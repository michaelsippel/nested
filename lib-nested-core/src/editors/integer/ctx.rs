
use {
    r3vi::{
        view::{OuterViewPort, singleton::*, list::*}
    },
    laddertypes::{TypeTerm, MorphismType},
    crate::{
        repr_tree::{ReprTree, ReprTreeExt, ReprLeaf, Context, GenericReprTreeMorphism},
        editors::{
            list::*,
            integer::*
        },
    },
    std::sync::{Arc, RwLock}
};

pub fn init_ctx(ctx: Arc<RwLock<Context>>) {
    // TODO: proper scoping
    ctx.write().unwrap().add_varname("SrcRadix");
    ctx.write().unwrap().add_varname("DstRadix");


    let posint_seq_morph_big_to_little = GenericReprTreeMorphism::new(
        Context::parse(&ctx, "ℕ ~ <PosInt Radix BigEndian>
                ~ <Seq <Digit Radix> ~ ℤ_2^64 ~ machine.UInt64 >"),
        Context::parse(&ctx, "ℕ ~ <PosInt Radix LittleEndian>
                ~ <Seq <Digit Radix> ~ ℤ_2^64 ~ machine.UInt64 >"),
        {
            let ctx = ctx.clone();
            move |src_rt, σ| {
                let src_digits = src_rt
                    .descend(Context::parse(&ctx, "
                          <PosInt Radix BigEndian>
                        ~ <Seq <Digit Radix> ~ ℤ_2^64 ~ machine.UInt64 >
                        ")
                        .apply_substitution(&|k|σ.get(k).cloned())
                        .clone()
                    ).expect("cant descend")
                       .view_seq::< u64 >();

                src_rt.attach_leaf_to(Context::parse(&ctx, "
                          <PosInt Radix LittleEndian>
                        ~ <Seq <Digit Radix> ~ ℤ_2^64 ~ machine.UInt64 >
                    ").apply_substitution(&|k|σ.get(k).cloned()).clone(),
                    src_digits.reverse()
                );            
            }
        }
    );

    let posint_list_morph_big_to_little = GenericReprTreeMorphism::new(
        Context::parse(&ctx, "ℕ ~ <PosInt Radix BigEndian>
                ~ <Seq~List <Digit Radix> ~ ℤ_2^64 ~ machine.UInt64 >"),
        Context::parse(&ctx, "ℕ ~ <PosInt Radix LittleEndian>
                ~ <Seq~List <Digit Radix> ~ ℤ_2^64 ~ machine.UInt64 >"),
        {
            let ctx = ctx.clone();
            move |src_rt, σ| {
                let src_digits = src_rt
                    .descend(Context::parse(&ctx, "
                          <PosInt Radix BigEndian>
                        ~ <Seq~List <Digit Radix> ~ ℤ_2^64 ~ machine.UInt64 >
                        ")
                        .apply_substitution(&|k|σ.get(k).cloned())
                        .clone()
                    ).expect("cant descend")
                       .view_list::< u64 >();

                src_rt.attach_leaf_to(Context::parse(&ctx, "
                          <PosInt Radix LittleEndian>
                        ~ <Seq~List <Digit Radix> ~ ℤ_2^64 ~ machine.UInt64 >
                    ").apply_substitution(&|k|σ.get(k).cloned()).clone(),
                    src_digits.reverse()
                );            
            }
        }
    );

    let posint_list_morph_little_to_big = GenericReprTreeMorphism::new(
        Context::parse(&ctx, "ℕ ~ <PosInt Radix LittleEndian>
                ~ <Seq~List <Digit Radix> ~ ℤ_2^64 ~ machine.UInt64 >"),
        Context::parse(&ctx, "ℕ ~ <PosInt Radix BigEndian>
                ~ <Seq~List <Digit Radix> ~ ℤ_2^64 ~ machine.UInt64 >"),
        {
            let ctx = ctx.clone();
            move |src_rt, σ| {
                let src_digits = src_rt
                    .descend(Context::parse(&ctx, "
                          <PosInt Radix LittleEndian>
                        ~ <Seq~List <Digit Radix> ~ ℤ_2^64 ~ machine.UInt64 >
                        ")
                        .apply_substitution(&|k|σ.get(k).cloned())
                        .clone()
                    ).expect("cant descend")
                       .view_list::< u64 >();

                src_rt.attach_leaf_to(Context::parse(&ctx, "
                          <PosInt Radix BigEndian>
                        ~ <Seq~List <Digit Radix> ~ ℤ_2^64 ~ machine.UInt64 >
                    ").apply_substitution(&|k|σ.get(k).cloned()).clone(),
                    src_digits.reverse()
                );            
            }
        }
    );


    let posint_list_morph_radix = GenericReprTreeMorphism::new(
        Context::parse(&ctx, "
              ℕ
            ~ <PosInt SrcRadix LittleEndian>
            ~ <Seq   <Digit SrcRadix>>
            ~ <List  <Digit SrcRadix>
                   ~ ℤ_2^64
                   ~ machine.UInt64>
        "),
        Context::parse(&ctx, "
              ℕ
            ~ <PosInt DstRadix LittleEndian>
            ~ <Seq   <Digit DstRadix>>
            ~ <List  <Digit DstRadix>
                   ~ ℤ_2^64
                   ~ machine.UInt64>
        "),
        {
            let ctx = ctx.clone();
            move |src_rt, σ| {
                let src_radix = match σ.get(&laddertypes::TypeID::Var(
                    ctx.read().unwrap().get_var_typeid("SrcRadix").unwrap()
                )) {
                    Some(laddertypes::TypeTerm::Num(n)) => *n as u64,
                    _ => 0
                };

                let dst_radix = match σ.get(&laddertypes::TypeID::Var(
                    ctx.read().unwrap().get_var_typeid("DstRadix").unwrap()
                )) {
                    Some(laddertypes::TypeTerm::Num(n)) => *n as u64,
                    _ => 0
                };

                let src_digits_rt = src_rt.descend(Context::parse(&ctx, "
                           <PosInt SrcRadix LittleEndian>
                         ~ <Seq <Digit SrcRadix>>
                         ~ <List <Digit SrcRadix>
                                 ~ ℤ_2^64
                                 ~ machine.UInt64 >
                    ").apply_substitution(&|k|σ.get(k).cloned()).clone()
                ).expect("cant descend repr tree");

                let dst_digits_port =
                    src_digits_rt.view_list::<u64>()
                        .to_sequence()
                        .to_positional_uint( src_radix )
                        .transform_radix( dst_radix )
                        .to_list();

                src_rt.attach_leaf_to(
                    Context::parse(&ctx, "
                        <PosInt DstRadix LittleEndian>
                        ~ <Seq <Digit DstRadix> >
                        ~ <List <Digit DstRadix>
                                ~ ℤ_2^64
                                ~ machine.UInt64 >
                    ").apply_substitution(&|k|σ.get(k).cloned()).clone(),
                    dst_digits_port
                );
            }
            
        }
    );

    ctx.write().unwrap().morphisms.add_morphism( posint_seq_morph_big_to_little );
    ctx.write().unwrap().morphisms.add_morphism( posint_list_morph_big_to_little );
    ctx.write().unwrap().morphisms.add_morphism( posint_list_morph_little_to_big );    
    ctx.write().unwrap().morphisms.add_morphism( posint_list_morph_radix );
}

