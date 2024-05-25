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


    /* Create a Representation-Tree of type `ℕ`
     */
    let mut rt_int = ReprTree::new_arc( Context::parse(&ctx, "ℕ") );

    /* Add a specific Representation-Path (big-endian hexadecimal)
     */
    rt_int.create_branch(
        Context::parse(&ctx, "<PosInt 16 BigEndian> ~ <Seq <Digit 16>> ~ <List <Digit 16>>")
    );

    /* Setup an Editor for the big-endian hexadecimal representation
     * (this will add the representation `<List <Digit 16>>~EditTree` to the ReprTree)
     */
    let rt_edittree_list = ctx.read().unwrap()
        .setup_edittree(
            rt_int.descend(Context::parse(&ctx, "
                      <PosInt 16 BigEndian>
                    ~ <Seq <Digit 16>>
                    ~ <List <Digit 16>>
            ")).expect("cant descend reprtree"),
            SingletonBuffer::new(0).get_port()
        );

    /* Setup a morphism to extract Char values from the list-editor
     */
    ctx.read().unwrap().morphisms.apply_morphism(
        rt_int.descend(Context::parse(&ctx, "
                      <PosInt 16 BigEndian>
                    ~ <Seq <Digit 16>>
                    ~ <List <Digit 16>>
        ")).expect("cant descend reprtree"),
        &Context::parse(&ctx, "<List <Digit 16>>~EditTree"),
        &Context::parse(&ctx, "<List <Digit 16>~Char>")
    );

    /*
     * map seq of chars to seq of u64 digits
     * and add this projection to the ReprTree
     */

    //
    //VVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVV
    let mut chars_view = rt_int.descend(Context::parse(&ctx, "
               < PosInt 16 BigEndian >
            ~  < Seq <Digit 16> >
            ~  < List <Digit 16>~Char >
        ")).expect("cant descend")
        .read().unwrap()
        .get_port::<dyn ListView<char>>()
        .unwrap();

    let mut digits_view = chars_view
        .to_sequence()
        .filter_map(
            |digit_char|

            /* TODO: call morphism for each item
             */
            match digit_char.to_digit(16) {
                Some(d) => Some(d as u64),
                None    => None
            }
        );

    rt_int.insert_leaf(Context::parse(&ctx, "
              <PosInt 16 BigEndian>
            ~ <Seq   <Digit 16>
                   ~ ℤ_2^64
                   ~ machine.UInt64 >
        "),
        nested::repr_tree::ReprLeaf::from_view( digits_view.clone() )
    );
    //ΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛ
    //


    /* convert to little endian
     */
    ctx.read().unwrap().morphisms.apply_morphism(
        rt_int.clone(),
        &Context::parse(&ctx, "ℕ ~ <PosInt 16 BigEndian> ~ <Seq <Digit 16>~ℤ_2^64~machine.UInt64>"),
        &Context::parse(&ctx, "ℕ ~ <PosInt 16 LittleEndian> ~ <Seq <Digit 16>~ℤ_2^64~machine.UInt64>")
    );

    /* convert to decimal
     */
    ctx.read().unwrap().morphisms.apply_morphism(
        rt_int.clone(),
        &Context::parse(&ctx, "ℕ ~ <PosInt 16 LittleEndian> ~ <Seq <Digit 16>~ℤ_2^64~machine.UInt64>"),
        &Context::parse(&ctx, "ℕ ~ <PosInt 10 LittleEndian> ~ <Seq <Digit 10>~ℤ_2^64~machine.UInt64>")
    );

    /* convert back to big endian
     */
    ctx.read().unwrap().morphisms.apply_morphism(
        rt_int.clone(),
        &Context::parse(&ctx, "ℕ ~ <PosInt 10 LittleEndian> ~ <Seq <Digit 10>~ℤ_2^64~machine.UInt64>"),
        &Context::parse(&ctx, "ℕ ~ <PosInt 10 BigEndian> ~ <Seq <Digit 10>~ℤ_2^64~machine.UInt64>")
    );

    /* map seq of u64 digits to seq of chars
     * and add this projection to the ReprTree
     */

    //
    //VVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVV
    let dec_digits_view =
        rt_int.descend(Context::parse(&ctx, "
                < PosInt 10 BigEndian >
              ~ < Seq  <Digit 10>
                      ~ ℤ_2^64
                      ~ machine.UInt64 >
        ")).expect("cant descend repr tree")
        .read().unwrap()
        .get_port::<dyn SequenceView<Item = u64>>().unwrap()
        .map(|digit| TerminalAtom::from(char::from_digit(*digit as u32, 10)))
        .to_grid_horizontal();
    //ΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛ
    //
    //VVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVV
    let hex_digits_view =
        rt_int.descend(Context::parse(&ctx, "
             < PosInt 16 BigEndian >
           ~ < Seq  <Digit 16>
                   ~ ℤ_2^64
                   ~ machine.UInt64 >
        ")).expect("cant descend")
        .read().unwrap()
        .view_seq::< u64 >()
        .map(|digit| TerminalAtom::from(char::from_digit(*digit as u32, 16)))
        .to_grid_horizontal();
    //ΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛΛ
    //



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

        comp.push(nested_tty::make_label("dec: ").offset(Vector2::new(3,4)));
        comp.push(dec_digits_view.offset(Vector2::new(8,4)).map_item(|_,a| {
            a.add_style_back(TerminalStyle::fg_color((30,90,200)))
        }));

        comp.push(nested_tty::make_label("hex: ").offset(Vector2::new(3,5)));
        comp.push(hex_digits_view.offset(Vector2::new(8,5)).map_item(|_,a| {
            a.add_style_back(TerminalStyle::fg_color((200, 200, 30)))
        }));
    }

    /* write the changes in the view of `term_port` to the terminal
     */
    app.show().await.expect("output error!");
}

