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
        edit_tree::{EditTree, TreeNav, TreeCursor}
    },
    nested_tty::{
        DisplaySegment, TTYApplication,
        TerminalCompositor, TerminalStyle, TerminalView,
        TerminalAtom, TerminalEvent
    },
    r3vi::{
        buffer::{singleton::*, vec::*},
        view::{port::UpdateTask, singleton::*, list::*, sequence::*},
        projection::*
    },
    std::sync::{Arc, RwLock},
};

fn setup_hex_master(ctx: &Arc<RwLock<Context>>, rt_int: &Arc<RwLock<ReprTree>>) {
    rt_int.write().unwrap().detach( ctx );
    ctx.read().unwrap().apply_morphism(
        rt_int,
        &laddertypes::MorphismType {
            src_type: Context::parse(&ctx, "ℕ ~ <PosInt 16 BigEndian> ~ <Seq <Digit 16>> ~ <List <Digit 16> ~ Char> ~ EditTree"),
            dst_type: Context::parse(&ctx, "ℕ ~ <PosInt 10 BigEndian> ~ <Seq <Digit 10>> ~ <List <Digit 10> ~ Char> ~ EditTree")
        }
    );
}

fn setup_dec_master(ctx: &Arc<RwLock<Context>>, rt_int: &Arc<RwLock<ReprTree>>) {
    rt_int.write().unwrap().detach( ctx );
    ctx.read().unwrap().apply_morphism(
        rt_int,
        &laddertypes::MorphismType {
            src_type: Context::parse(&ctx, "ℕ ~ <PosInt 10 BigEndian> ~ <Seq <Digit 10>> ~ <List <Digit 10> ~ Char> ~ EditTree"),
            dst_type: Context::parse(&ctx, "ℕ ~ <PosInt 16 BigEndian> ~ <Seq <Digit 16>> ~ <List <Digit 16> ~ Char> ~ EditTree")
        }
    );
}

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
     * with a specific representation-path (big-endian hexadecimal string)
     */
    let mut rt_int = nested::repr_tree::ReprTree::from_str(
        /* TYPE */
        Context::parse(&ctx, "
              ℕ
            ~ <PosInt 16 BigEndian>
            ~ <Seq <Digit 16>>
            ~ <List <Digit 16>>
            ~ <List Char>
            ~ <Vec Char>
        "),

        /* VALUE */
        "cff"
    );

    /* initially copy values from Vec to EditTree...
     */
    ctx.read().unwrap().apply_morphism(
        &rt_int,
        &nested::repr_tree::morphism::MorphismType {
            src_type: Context::parse(&ctx, "ℕ ~ <PosInt 16 BigEndian> ~ <Seq <Digit 16>> ~ <List <Digit 16> ~ Char> ~ <Vec Char>"),
            dst_type: Context::parse(&ctx, "ℕ ~ <PosInt 16 BigEndian> ~ <Seq <Digit 16>> ~ <List <Digit 16> ~ Char> ~ EditTree")
        });

    setup_hex_master(&ctx, &rt_int);

    let edittree_hex_be_list = ctx.read().unwrap()
        .setup_edittree(
            rt_int.descend(Context::parse(&ctx,"
                <PosInt 16 BigEndian>
                ~ <Seq <Digit 16>>
                ~ <List <Digit 16>>
                ~ <List Char>
            ")).expect("descend"),
            SingletonBuffer::new(0).get_port()
        ).unwrap().get();

    let edittree_dec_be_list = ctx.read().unwrap()
        .setup_edittree(
            rt_int.descend(Context::parse(&ctx,"
                <PosInt 10 BigEndian>
                ~ <Seq <Digit 10>>
                ~ <List <Digit 10>>
                ~ <List Char>
            ")).expect("descend"),
            SingletonBuffer::new(0).get_port()
        ).unwrap().get();

    let hex_digits_view = rt_int.descend(Context::parse(&ctx, "
              <PosInt 16 LittleEndian>
            ~ <Seq <Digit 16> >
            ~ <List <Digit 16>
                    ~ ℤ_2^64
                    ~ machine.UInt64 >
        ")).expect("descend")
        .view_list::<u64>()
        .map(|v| TerminalAtom::from(char::from_digit(*v as u32, 16)))
        .to_sequence()
        .to_grid_horizontal();

    let dec_digits_view = rt_int.descend(Context::parse(&ctx, "
              <PosInt 10 LittleEndian>
            ~ <Seq <Digit 10>>
            ~ <List <Digit 10>
                    ~ ℤ_2^64
                    ~ machine.UInt64 >
        ")).expect("descend")
        .view_list::<u64>()
        .map(|v| TerminalAtom::from(char::from_digit(*v as u32, 10)))
        .to_sequence()
        .to_grid_horizontal();

    /* list of both editors
     */
    let mut list_editor = nested::editors::list::ListEditor::new(ctx.clone(), Context::parse(&ctx, "<Seq Char>"));
    list_editor.data.push( edittree_hex_be_list.clone() );
    list_editor.data.push( edittree_dec_be_list.clone() );
    let mut edittree = list_editor.into_node(SingletonBuffer::new(0).get_port());

    /* cursors are a bit screwed initially so fix them up
     * TODO: how to fix this generally?
     */
    edittree_hex_be_list.write().unwrap().goto(TreeCursor::none());
    edittree_dec_be_list.write().unwrap().goto(TreeCursor::none());
    edittree.goto(TreeCursor{
        leaf_mode: nested::editors::list::ListCursorMode::Insert,
        tree_addr: vec![0,0]
    });
    let edittree = Arc::new(RwLock::new(edittree));
 
    /* setup terminal
     */
    let app = TTYApplication::new({
        /* event handler
         */
        let ctx = ctx.clone();
        let rt_int = rt_int.clone();
        let last_idx = RwLock::new(1);
        move |ev| {

            let cur = edittree.read().unwrap().get_cursor();
            if cur.tree_addr.len() > 0 {
                match cur.tree_addr[0] {
                    0 => {
                        let mut li = last_idx.write().unwrap();
                        if *li != 0 {
                            setup_hex_master(&ctx, &rt_int);
                            *li = 0;
                        }
                    }
                    1 => {
                        let mut li = last_idx.write().unwrap();
                        if *li != 1 {
                            setup_dec_master(&ctx, &rt_int);
                            *li = 1;
                        }
                    }
                    _=>{}
                }
            }

            edittree.write().unwrap().send_cmd_obj(ev.to_repr_tree(&ctx));
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

        fn show_edit_tree( ctx: &Arc<RwLock<Context>>, comp: &mut TerminalCompositor, rt: &Arc<RwLock<ReprTree>>, y: i16 )
        {
            let rt_edittree = rt.descend(Context::parse(&ctx, "EditTree")).expect("descend");
            let halo_type = rt_edittree.read().unwrap().get_halo_type().clone();
            let edittree = rt_edittree.read().unwrap().get_view::<dyn r3vi::view::singleton::SingletonView<Item = Arc<RwLock<EditTree>>>>().unwrap().get().read().unwrap().clone();

            comp.push(  nested_tty::make_label( &ctx.read().unwrap().type_term_to_str(&halo_type) ) 
                .map_item(|_pt, atom| atom.add_style_front(TerminalStyle::fg_color((90,90,90))))
                .offset(Vector2::new(1,y)));

            comp.push(  edittree.display_view()
                .offset(Vector2::new(1,y+1)));
        }

        show_edit_tree(&ctx, &mut comp, &rt_int.descend(Context::parse(&ctx, "<PosInt 16 BigEndian> ~ <Seq~List <Digit 16>~Char>")).expect(""), 1);
        show_edit_tree(&ctx, &mut comp, &rt_int.descend(Context::parse(&ctx, "<PosInt 10 BigEndian> ~ <Seq~List <Digit 10>~Char>")).expect(""), 4);

        /* project the seq of u64 representations to a view
         */
        comp.push(nested_tty::make_label("dec: ").offset(Vector2::new(3,7)));
        comp.push(dec_digits_view.offset(Vector2::new(8,7)).map_item(|_,a| {
            a.add_style_back(TerminalStyle::fg_color((30,90,200)))
        }));

        comp.push(nested_tty::make_label("hex: ").offset(Vector2::new(3,8)));
        comp.push(hex_digits_view.offset(Vector2::new(8,8)).map_item(|_,a| {
            a.add_style_back(TerminalStyle::fg_color((200, 200, 30)))
        }));
    }

    /* write the changes in the view of `term_port` to the terminal
     */
    app.show().await.expect("output error!");
}

