//! In the following example, a <List Char> editor
//! as before is used, but its data is morphed into
//! representing a positional integer which is then
//! projected into different radices and displayed
//! in different views on screen

extern crate cgmath;
extern crate nested;
extern crate nested_tty;
extern crate r3vi;
extern crate termion;

use {
    cgmath::Vector2,
    nested::{
        editors::{
            ObjCommander
        },
        repr_tree::{Context, ReprTree, ReprTreeExt, ReprLeaf},
        edit_tree::{EditTree}
    },
    nested_tty::{
        DisplaySegment, TTYApplication,
        TerminalCompositor, TerminalStyle, TerminalView,
        TerminalAtom, TerminalEvent
    },
    r3vi::{
        buffer::{singleton::*, vec::*},
        view::{port::UpdateTask, list::*, sequence::*},
        projection::*
    },
    std::sync::{Arc, RwLock},
};

#[async_std::main]
async fn main() {
    /* setup context
     */
    let ctx = Arc::new(RwLock::new(Context::new()));
    nested::editors::char::init_ctx( ctx.clone() );
    nested::editors::digit::init_ctx( ctx.clone() );
    nested::editors::integer::init_ctx( ctx.clone() );
    nested::editors::list::init_ctx( ctx.clone() );
    nested_tty::setup_edittree_hook(&ctx);

    /* Create a Representation-Tree of type <List <Digit 16>>
     */
    let mut rt_digitlist = ReprTree::new_arc( Context::parse(&ctx, "<List <Digit 16>>") );
    let mut rt_digitseq = ReprTree::new_arc( Context::parse(&ctx, "<Seq <Digit 16>>") );
    rt_digitseq.insert_branch( rt_digitlist );
    let mut rt_posint = ReprTree::new_arc(Context::parse(&ctx, "<PosInt 16 BigEndian>"));
    rt_posint.insert_branch( rt_digitseq );
    let mut rt_int = ReprTree::new_arc( Context::parse(&ctx, "ℕ") );
    rt_int.insert_branch( rt_posint );

    /* Setup an Editor for this ReprTree
     * (this will add the representation <List <Digit 16>>~EditTree to the ReprTree)
     */
    let rt_edittree_list = ctx.read().unwrap()
        .setup_edittree(
            ReprTree::descend(
                &rt_int,
                Context::parse(&ctx, "<PosInt 16 BigEndian>~<Seq~List <Digit 16>>")
            ).expect("cant descend reprtree"),
            SingletonBuffer::new(0).get_port()
        );

    ctx.read().unwrap().morphisms.apply_morphism(
        ReprTree::descend(&rt_int,
            Context::parse(&ctx, "
                    <PosInt 16 BigEndian>
                    ~<Seq <Digit 16>>
                    ~<List <Digit 16>>
                ")
        ).expect("cant descend repr tree"),
        &Context::parse(&ctx, "<List <Digit 16>>~EditTree"),
        &Context::parse(&ctx, "<List <Digit 16>~Char>")
    );

    /*
     * map seq of chars to seq of u64 digits
     */
    let mut chars_view =
        ReprTree::descend(
            &rt_int,
            Context::parse(&ctx, "<PosInt 16 BigEndian>~<Seq <Digit 16>>~<List <Digit 16>~Char>")
        ).expect("cant descend")
        .read().unwrap()
        .get_port::<dyn ListView<char>>()
        .unwrap();

    let mut digits_view = chars_view
        .to_sequence()
        .filter_map(
            |digit_char|

            /* TODO: call morphism
             */
            match digit_char.to_digit(16) {
                Some(d) => Some(d as usize),
                None    => None
            }
        );

    rt_int.write().unwrap().insert_leaf(
        vec![
            Context::parse(&ctx, "<PosInt 16 BigEndian>"),
            Context::parse(&ctx, "<Seq <Digit 16>>"),
            Context::parse(&ctx, "<Seq ℤ_2^64>"),
            Context::parse(&ctx, "<Seq machine.UInt64>")
        ].into_iter(),
        nested::repr_tree::ReprLeaf::from_view( digits_view.clone() )
    );
    //
    //

    ctx.read().unwrap().morphisms.apply_morphism(
        rt_int.clone(),
        &Context::parse(&ctx, "ℕ ~ <PosInt 16 BigEndian> ~ <Seq <Digit 16>~ℤ_2^64~machine.UInt64>"),
        &Context::parse(&ctx, "ℕ ~ <PosInt 16 LittleEndian> ~ <Seq <Digit 16>~ℤ_2^64~machine.UInt64>")
    );
    ctx.read().unwrap().morphisms.apply_morphism(
        rt_int.clone(),
        &Context::parse(&ctx, "ℕ ~ <PosInt 16 LittleEndian> ~ <Seq <Digit 16>~ℤ_2^64~machine.UInt64>"),
        &Context::parse(&ctx, "ℕ ~ <PosInt 10 LittleEndian> ~ <Seq <Digit 10>~ℤ_2^64~machine.UInt64>")
    );
    ctx.read().unwrap().morphisms.apply_morphism(
        rt_int.clone(),
        &Context::parse(&ctx, "ℕ ~ <PosInt 10 LittleEndian> ~ <Seq <Digit 10>~ℤ_2^64~machine.UInt64>"),
        &Context::parse(&ctx, "ℕ ~ <PosInt 10 BigEndian> ~ <Seq <Digit 10>~ℤ_2^64~machine.UInt64>")
    );

    let dec_digits_view = ReprTree::descend(&rt_int,
        Context::parse(&ctx, "
                <PosInt 10 BigEndian>
                ~< Seq <Digit 10>~ℤ_2^64~machine.UInt64 >
        ")
    ).expect("cant descend repr tree")
        .read().unwrap()
        .get_port::<dyn SequenceView<Item = usize>>().unwrap()
        .map(
            /* TODO: call morphism
             */
            |digit| {
                TerminalAtom::from(
                    char::from_digit(*digit as u32, 10)
                )
            }
        )
        .to_grid_horizontal();

    let hex_digits_view =
        ReprTree::descend(
            &rt_int,
            Context::parse(&ctx, "
                <PosInt 16 BigEndian>
                ~<Seq  <Digit 16>  >
                ~<List <Digit 16>
                       ~Char>")
        ).expect("cant descend")
        .read().unwrap()
        .get_port::<dyn ListView<char>>().unwrap()
        .to_sequence()
        .to_grid_horizontal()
        .map_item(|_pt,c| TerminalAtom::new(*c, TerminalStyle::fg_color((30,90,200))));

    /* setup terminal
     */
    let app = TTYApplication::new({
        let edittree_list = rt_edittree_list.clone();

        /* event handler
         */
        let ctx = ctx.clone();
        move |ev| {
            edittree_list.get().send_cmd_obj(ev.to_repr_tree(&ctx));
        }
    });

    /* Setup the compositor to serve as root-view
     * by routing it to the `app.port` Viewport,
     * so it will be displayed on TTY-output.
     */
    let compositor = TerminalCompositor::new(app.port.inner());

    /* Now add some views to our compositor
     */
    {
        let mut comp = compositor.write().unwrap();

        let label_str = ctx.read().unwrap().type_term_to_str(&rt_int.read().unwrap().get_type());
        comp.push(
            nested_tty::make_label(&label_str)
                .map_item(|_pt, atom| atom.add_style_front(TerminalStyle::fg_color((90,90,90))))
                .offset(Vector2::new(1,1)));

        comp.push(rt_edittree_list.get()
                .display_view()
                .offset(Vector2::new(3,2)));

        comp.push(dec_digits_view.offset(Vector2::new(3,4)));
        comp.push(hex_digits_view.offset(Vector2::new(3,5)));
    }

    /* write the changes in the view of `term_port` to the terminal
     */
    app.show().await.expect("output error!");
}
