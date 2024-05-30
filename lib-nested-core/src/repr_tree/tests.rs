
use {
    r3vi::{
        buffer::singleton::{
            SingletonBuffer
        },
        view::port::UpdateTask
    },
    crate::{
        repr_tree::{Context, ReprTreeExt, ReprTree, ReprLeaf}
    },
    std::sync::{Arc, RwLock}
};

#[test]
fn halo_type() {
    let ctx = Arc::new(RwLock::new(Context::new()));

    let mut rt1 = ReprTree::new_arc(Context::parse(&ctx, "ℕ"));
    let mut rt2 = ReprTree::new_arc(Context::parse(&ctx, "<PosInt 10 BigEndian>"));
    rt1.insert_branch( rt2.clone() );
    assert_eq!( rt2.read().unwrap().get_halo_type(), &Context::parse(&ctx, "ℕ") );
    
    let mut rt3 = ReprTree::new_arc(Context::parse(&ctx, "<Seq <Digit 10>>"));
    rt2.insert_branch( rt3.clone() );
    assert_eq!( rt3.read().unwrap().get_halo_type(), &Context::parse(&ctx, "ℕ~<PosInt 10 BigEndian>") );

    let rt4 = ReprTree::new_arc(Context::parse(&ctx, "<List <Digit 10>>"));
    rt3.insert_branch( rt4.clone() );
    assert_eq!( rt4.read().unwrap().get_halo_type(), &Context::parse(&ctx, "ℕ~<PosInt 10 BigEndian>~<Seq <Digit 10>>") );

    
    let mut r = ReprTree::new_arc(Context::parse(&ctx, "ℕ"));
    r.create_branch(Context::parse(&ctx, "<PosInt 10 BigEndian>~<Seq <Digit 10>>"));
    assert_eq!( r.descend(Context::parse(&ctx, "<PosInt 10 BigEndian>")).unwrap().read().unwrap().get_halo_type(), &Context::parse(&ctx, "ℕ") );
    assert_eq!( r.descend(Context::parse(&ctx, "<PosInt 10 BigEndian>~<Seq <Digit 10>>")).unwrap().read().unwrap().get_halo_type(), &Context::parse(&ctx, "ℕ~<PosInt 10 BigEndian>") );
}

#[test]
fn char_view() {
    let ctx = Arc::new(RwLock::new(Context::new()));
    crate::editors::digit::init_ctx( ctx.clone() );

    let mut rt_digit = ReprTree::new_arc( Context::parse(&ctx, "<Digit 16>") );
    rt_digit.insert_leaf(
        Context::parse(&ctx, "Char"),
        ReprLeaf::from_singleton_buffer( SingletonBuffer::new('5') )
    );

    //<><><><>
    let mut digit_char_buffer = rt_digit
        .descend( Context::parse(&ctx, "Char") ).unwrap()
        .singleton_buffer::<char>();

    assert_eq!( digit_char_buffer.get(), '5' );
    //<><><><>

    let digit_char_view = rt_digit
        .descend(Context::parse(&ctx, "Char")).unwrap()
        .view_char();

    assert_eq!( digit_char_view.get_view().unwrap().get(), '5' );


    //<><><><>
    // `Char-view` is correctly coupled to `char-buffer`
    digit_char_buffer.set('2');
    assert_eq!( digit_char_view.get_view().unwrap().get(), '2' );
}

#[test]
fn digit_projection_char_to_u64() {
    let ctx = Arc::new(RwLock::new(Context::new()));
    crate::editors::digit::init_ctx( ctx.clone() );

    let mut rt_digit = ReprTree::new_arc( Context::parse(&ctx, "<Digit 16>") );

    rt_digit.insert_leaf(
        Context::parse(&ctx, "Char"),
        ReprLeaf::from_singleton_buffer( SingletonBuffer::new('5') )
    );

    //<><><><>
    // add another representation
 
    ctx.read().unwrap().morphisms.apply_morphism(
        rt_digit.clone(),
        &Context::parse(&ctx, "<Digit 16>~Char"),
        &Context::parse(&ctx, "<Digit 16>~ℤ_2^64~machine.UInt64")
    );

    let digit_u64_view = rt_digit
        .descend(Context::parse(&ctx, "ℤ_2^64~machine.UInt64")).unwrap()
        .view_u64();

    assert_eq!( digit_u64_view.get_view().unwrap().get(), 5 as u64 );


    // projection behaves accordingly , when buffer is changed

    let mut digit_char_buffer = rt_digit
        .descend( Context::parse(&ctx, "Char") ).unwrap()
        .singleton_buffer::<char>();

    digit_char_buffer.set('2');
    assert_eq!( digit_u64_view.get_view().unwrap().get(), 2 as u64 );
}

#[test]
fn digit_projection_u64_to_char() {
    let ctx = Arc::new(RwLock::new(Context::new()));
    crate::editors::digit::init_ctx( ctx.clone() );

    let mut rt_digit = ReprTree::new_arc( Context::parse(&ctx, "<Digit 16>") );

    rt_digit.insert_leaf(
        Context::parse(&ctx, "ℤ_2^64~machine.UInt64"),
        ReprLeaf::from_singleton_buffer( SingletonBuffer::new(5 as u64) )
    );

    //<><><><>
    // add another representation
 
    ctx.read().unwrap().morphisms.apply_morphism(
        rt_digit.clone(),
        &Context::parse(&ctx, "<Digit 16>~ℤ_2^64~machine.UInt64"),
        &Context::parse(&ctx, "<Digit 16>~Char")
    );

    let digit_u64_view = rt_digit
        .descend(Context::parse(&ctx, "Char")).unwrap()
        .view_char();

    assert_eq!( digit_u64_view.get_view().unwrap().get(), '5' );
}


#[test]
fn char_buffered_projection() {
    let ctx = Arc::new(RwLock::new(Context::new()));
    crate::editors::digit::init_ctx( ctx.clone() );

    let mut rt_digit = ReprTree::new_arc( Context::parse(&ctx, "<Digit 16>") );

    rt_digit.insert_leaf(
        Context::parse(&ctx, "ℤ_2^64~machine.UInt64"),
        ReprLeaf::from_singleton_buffer( SingletonBuffer::new(8 as u64) )
    );

    let mut digit_u64_buffer = rt_digit
        .descend(Context::parse(&ctx, "ℤ_2^64~machine.UInt64")).unwrap()
        .singleton_buffer::<u64>();

    assert_eq!( digit_u64_buffer.get(), 8 );

    rt_digit.insert_leaf(
        Context::parse(&ctx, "Char"),
        ReprLeaf::from_singleton_buffer( SingletonBuffer::new('5') )
    );

    let digit_char_buf = rt_digit
        .descend(Context::parse(&ctx, "Char")).unwrap()
        .singleton_buffer::<char>();
    let digit_char_view = rt_digit
        .descend(Context::parse(&ctx, "Char")).unwrap()
        .view_char();

    // before setting up the morphism, char-view remains as initialized
    assert_eq!( digit_char_buf.get(), '5' );
    assert_eq!( digit_char_view.get_view().unwrap().get(), '5' );

    // now we attach the char-repr to the u64-repr
    ctx.read().unwrap().morphisms.apply_morphism(
        rt_digit.clone(),
        &Context::parse(&ctx, "<Digit 16>~ℤ_2^64~machine.UInt64"),
        &Context::parse(&ctx, "<Digit 16>~Char")
    );

    // char buffer and view should now follow the u64-buffer
    assert_eq!( digit_char_view.get_view().unwrap().get(), '8' );
    assert_eq!( digit_char_buf.get(), '8' );

    // now u64-buffer changes, and char-buffer should change accordingly
    digit_u64_buffer.set(3);
    assert_eq!( digit_u64_buffer.get(), 3 );

    // char buffer should follow
    digit_char_view.0.update();
    assert_eq!( digit_char_buf.get(), '3' );
    assert_eq!( digit_char_view.get_view().unwrap().get(), '3' ); 
}

