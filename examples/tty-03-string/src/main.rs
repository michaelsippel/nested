//! Similarly to `tty-02-digit`, a editor is created
//! but of type <List Char>.
//! The contents of the editor can be retrieved by
//! a morphism from the `EditTree` node.
//! To demonstrate that, the values are are mapped
//! to the TTY-display in different form.

extern crate cgmath;
extern crate nested;
extern crate nested_tty;
extern crate r3vi;
extern crate termion;

use {
    cgmath::Vector2,
    nested::{
        editors::ObjCommander,
        repr_tree::{Context, ReprTree, ReprTreeExt},
        edit_tree::{EditTree}
    },
    nested_tty::{
        DisplaySegment, TTYApplication,
        TerminalCompositor, TerminalStyle, TerminalView,
        TerminalAtom, TerminalEvent
    },
    r3vi::{
        buffer::{singleton::*, vec::*},
        view::{port::UpdateTask, list::*, sequence::SequenceViewExt}
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

    /* Create a Representation-Tree of type <List Char>
     */
    let mut rt_string = ReprTree::from_str(
        Context::parse(&ctx, "<List Char>~<Vec Char>"),
        "hello world"
    );

    /* create EditTree
     */
    ctx.read().unwrap().apply_morphism(
        &rt_string,
        &laddertypes::MorphismType {
            src_type: Context::parse(&ctx, "<List~Vec Char>"),
            dst_type: Context::parse(&ctx, "<List Char> ~ EditTree")
        }
    );

    // .. avoid cycle of projections..
    rt_string.write().unwrap().detach(&ctx);

    /* Setup the Editor-View for this ReprTree
     */
    let edittree_list = ctx.read().unwrap()
        .setup_edittree(
            rt_string.clone(),
            SingletonBuffer::new(0).get_port()
        ).unwrap();

    /* In order to get access to the values that are modified by the Editor,
     * we apply a morphism that, given the List of Edit-Trees, extracts
     * the value from each EditTree and shows them in a ListView.
     */
    ctx.read().unwrap().apply_morphism(
        &rt_string,
        &laddertypes::MorphismType {
            src_type: Context::parse(&ctx, "<List Char>~EditTree"),
            dst_type: Context::parse(&ctx, "<List Char>")
        }
    );

    /* Now, get the ListView that serves our char-values.
     * This view is a projection created by the morphism that was called above.
     */
    let mut chars_view = rt_string
        .read().unwrap()
        .get_port::<dyn ListView<char>>()
        .unwrap();

    /* Lets add another morphism which will store the values
     * of the character-list in a `Vec<char>`
     */
    ctx.read().unwrap().apply_morphism(
        &rt_string,
        &laddertypes::MorphismType {
            src_type: Context::parse(&ctx, "<List Char>"),
            dst_type: Context::parse(&ctx, "<List Char>~<Vec Char>")
        }
    );

    /* Access the Vec<char> object (wrapped behind a VecBuffer<char>)
     * from the ReprTree.
     */
    let chars_vec = rt_string
        .descend(Context::parse(&ctx, "<Vec Char>")).unwrap()
        .vec_buffer::<char>();

    /* transform `ListView<char>` into a `TerminalView`
     */
    let string_view_tty = chars_view
        .to_sequence()
        .to_grid_vertical()
        .map_item(|_pt,c| TerminalAtom::new(*c, TerminalStyle::fg_color((200,10,60))));

    /* setup terminal
     */
    let app = TTYApplication::new({
        let edittree_list = edittree_list.clone();

        /* event handler
         */
        let ctx = ctx.clone();
        move |ev| {
            edittree_list.get().write().unwrap().send_cmd_obj(ev.to_repr_tree(&ctx));
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

        let label_str = ctx.read().unwrap().type_term_to_str(&rt_string.read().unwrap().get_type());
        comp.push(
            nested_tty::make_label(&label_str)
                .map_item(|_pt, atom| atom.add_style_front(TerminalStyle::fg_color((90,90,90))))
                .offset(Vector2::new(1,1)));

        comp.push(
            edittree_list.get()
                .read().unwrap()
                .display_view()
                .offset(Vector2::new(3,2)));

        comp.push(
            string_view_tty
                .offset(Vector2::new(5,3)));
    }

    /* write the changes in the view of `term_port` to the terminal
     */
    app.show().await.expect("output error!");

    /* need to call update because changes are applied lazily
     */
    chars_vec.get_port().0.update();

    /* Vec<char> to String
     */
    let string = chars_vec
        .get_port()
        .to_sequence()
        .get_view().unwrap()
        //.data.read().unwrap()
        .iter().collect::<String>();

    eprintln!("value of the editor was: {}\n\n", string);
}