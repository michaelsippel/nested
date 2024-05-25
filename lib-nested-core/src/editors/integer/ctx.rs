
use {
    r3vi::{
        view::{OuterViewPort, singleton::*}
    },
    laddertypes::{TypeTerm},
    crate::{
        repr_tree::{ReprTree, ReprLeaf, Context, MorphismType},
        editors::{
            list::*,
            integer::*
        },
    },
    std::sync::{Arc, RwLock}
};

pub fn init_ctx(ctx: Arc<RwLock<Context>>) {
    // TODO: proper scoping
    // ctx.write().unwrap().add_varname("Radix");
    ctx.write().unwrap().add_varname("SrcRadix");
    ctx.write().unwrap().add_varname("DstRadix");

    let morphism_type = MorphismType {
        src_type: Context::parse(&ctx, "ℕ ~ <PosInt Radix BigEndian> ~ <Seq <Digit SrcRadix>~ℤ_2^64~machine.UInt64>"),
        dst_type: Context::parse(&ctx, "ℕ ~ <PosInt Radix LittleEndian> ~ <Seq <Digit DstRadix>~ℤ_2^64~machine.UInt64>")
    };

    ctx.write().unwrap().morphisms.add_morphism(
        morphism_type,
        {
            let ctx = ctx.clone();
            move |src_rt, σ| {
                let src_digits = ReprTree::descend(
                    &src_rt,
                    Context::parse(&ctx, "
                            <PosInt Radix BigEndian>
                            ~<Seq <Digit Radix>~ℤ_2^64~machine.UInt64 >
                        ")
                    .apply_substitution(&|k|σ.get(k).cloned()).clone()
                ).expect("cant descend")
                    .read().unwrap()
                    .view_seq::< u64 >();

                src_rt.write().unwrap().insert_leaf(
                        vec![
                            Context::parse(&ctx, "<PosInt Radix LittleEndian>")
                                .apply_substitution(&|k|σ.get(k).cloned()).clone(),
                            Context::parse(&ctx, "<Seq <Digit Radix>>")
                                .apply_substitution(&|k|σ.get(k).cloned()).clone(),
                            Context::parse(&ctx, "<Seq ℤ_2^64>"),
                            Context::parse(&ctx, "<Seq machine.UInt64>")
                        ].into_iter(),

                        ReprLeaf::from_view( src_digits.reverse() )
                    );
            }
        }
    );






    let morphism_type = MorphismType {
        src_type: Context::parse(&ctx, "ℕ ~ <PosInt Radix LittleEndian> ~ <Seq <Digit SrcRadix>~ℤ_2^64~machine.UInt64>"),
        dst_type: Context::parse(&ctx, "ℕ ~ <PosInt Radix BigEndian> ~ <Seq <Digit DstRadix>~ℤ_2^64~machine.UInt64>")
    };

    ctx.write().unwrap().morphisms.add_morphism(
        morphism_type,
        {
            let ctx = ctx.clone();
            move |src_rt, σ| {
                let src_digits = ReprTree::descend(
                    &src_rt,
                    Context::parse(&ctx, "
                            <PosInt Radix LittleEndian>
                            ~<Seq <Digit Radix>~ℤ_2^64~machine.UInt64 >
                        ")
                    .apply_substitution(&|k|σ.get(k).cloned()).clone()
                ).expect("cant descend")
                    .read().unwrap()
                    .view_seq::< u64 >();

                src_rt.write().unwrap().insert_leaf(
                        vec![
                            Context::parse(&ctx, "<PosInt Radix BigEndian>")
                                .apply_substitution(&|k|σ.get(k).cloned()).clone(),
                            Context::parse(&ctx, "<Seq <Digit Radix>>")
                                .apply_substitution(&|k|σ.get(k).cloned()).clone(),
                            Context::parse(&ctx, "<Seq ℤ_2^64>"),
                            Context::parse(&ctx, "<Seq machine.UInt64>")
                        ].into_iter(),

                        ReprLeaf::from_view( src_digits.reverse() )
                    );
            }
        }
    );




    let morphism_type = MorphismType {
        src_type: Context::parse(&ctx, "ℕ ~ <PosInt SrcRadix LittleEndian> ~ <Seq <Digit SrcRadix>~ℤ_2^64~machine.UInt64>"),
        dst_type: Context::parse(&ctx, "ℕ ~ <PosInt DstRadix LittleEndian> ~ <Seq <Digit DstRadix>~ℤ_2^64~machine.UInt64>")
    };

    ctx.write().unwrap().morphisms.add_morphism(
        morphism_type,
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

                let src_digits_rt = ReprTree::descend(
                    src_rt,
                    Context::parse(&ctx, "
                           <PosInt SrcRadix LittleEndian>
                         ~ <Seq <Digit SrcRadix> ~ ℤ_2^64 ~ machine.UInt64 >"
                    ).apply_substitution(&|k|σ.get(k).cloned()).clone()
                ).expect("cant descend repr tree");

                let dst_digits_port =
                    src_digits_rt.read().unwrap()
                        .view_seq::<u64>()
                        .to_positional_uint( src_radix )
                        .transform_radix( dst_radix )
                ;

                src_rt.write().unwrap()
                    .insert_leaf(
                        vec![
                            Context::parse(&ctx, "<PosInt DstRadix LittleEndian>").apply_substitution(&|k|σ.get(k).cloned()).clone(),
                            Context::parse(&ctx, "<Seq <Digit DstRadix>>").apply_substitution(&|k|σ.get(k).cloned()).clone(),
                            Context::parse(&ctx, "<Seq ℤ_2^64>"),
                            Context::parse(&ctx, "<Seq machine.UInt64>"),
                        ].into_iter(),
                        ReprLeaf::from_view(dst_digits_port)
                    );
            }
        }
    );
}

