
use {
    r3vi::{
        view::{OuterViewPort, singleton::*, list::*}
    },
    laddertypes::{TypeTerm},
    crate::{
        repr_tree::{ReprTree, ReprTreeExt, ReprLeaf, Context, MorphismType},
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


    /*
     *    MACHINE INT,  SEQ
     */
    let morphism_type =
        MorphismType {
            src_type: Context::parse(&ctx, "
                  ℕ
                ~ <PosInt Radix BigEndian>
                ~ <Seq   <Digit Radix>
                       ~ ℤ_2^64
                       ~ machine.UInt64 >"),
            dst_type: Context::parse(&ctx, "
                  ℕ
                ~ <PosInt Radix LittleEndian>
                ~ <Seq   <Digit Radix>
                       ~ ℤ_2^64
                       ~ machine.UInt64 >")
        };
    ctx.write().unwrap().morphisms.add_morphism(
        morphism_type, {
            let ctx = ctx.clone();
            move |src_rt, σ| {
                let src_digits = src_rt.descend(
                    Context::parse(&ctx, "
                          <PosInt Radix BigEndian>
                        ~ <Seq   <Digit Radix>
                               ~ ℤ_2^64
                               ~ machine.UInt64 >
                    ")
                        .apply_substitution(&|k|σ.get(k).cloned())
                        .clone()
                ).expect("cant descend")
                   .view_seq::< u64 >();

                src_rt.attach_leaf_to(Context::parse(&ctx, "
                          <PosInt Radix LittleEndian>
                        ~ <Seq <Digit Radix>
                               ~ ℤ_2^64
                               ~ machine.UInt64 >
                    ").apply_substitution(&|k|σ.get(k).cloned()).clone(),
                    src_digits.reverse()
                );
            }
        }
    );

    /*    MACHINE INT,   LIST
     */
    let morphism_type = MorphismType {
        src_type: Context::parse(&ctx, "
              ℕ
            ~ <PosInt Radix BigEndian>
            ~ <Seq <Digit Radix>>
            ~ <List <Digit Radix>>
            ~ <List ℤ_2^64>
            ~ <List machine.UInt64>
        "),
        dst_type: Context::parse(&ctx, "
              ℕ 
            ~ <PosInt Radix LittleEndian>
            ~ <Seq <Digit Radix>>
            ~ <List <Digit Radix>>
            ~ <List ℤ_2^64>
            ~ <List machine.UInt64>
        ")
    };

    ctx.write().unwrap().morphisms.add_morphism(
        morphism_type,
        {
            let ctx = ctx.clone();
            move |src_rt, σ| {
                let src_digits = src_rt.descend(Context::parse(&ctx, "
                              <PosInt Radix BigEndian>
                            ~ <Seq <Digit Radix>>
                            ~ <List <Digit Radix> ~ ℤ_2^64 ~ machine.UInt64 >
                        ")
                    .apply_substitution(&|k|σ.get(k).cloned()).clone()
                ).expect("cant descend")
                .get_port::< dyn ListView<u64> >().unwrap();

                src_rt.attach_leaf_to(
                    Context::parse(&ctx, "
                              <PosInt Radix LittleEndian>
                            ~ <Seq <Digit Radix>>
                            ~ <List <Digit Radix> ~ ℤ_2^64 ~ machine.UInt64>
                    ").apply_substitution(&|k| σ.get(k).cloned()).clone(),
                    src_digits.reverse()
                );
            }
        }
    );


    let mt = MorphismType {
        src_type: Context::parse(&ctx, "
            ℕ
            ~ <PosInt Radix BigEndian>
            ~ <Seq <Digit Radix>>
            ~ <List <Digit Radix>>
            ~ <List Char>
        "),
        dst_type: Context::parse(&ctx, "
            ℕ
            ~ <PosInt Radix LittleEndian>
            ~ <Seq <Digit Radix>>
            ~ <List <Digit Radix>>
            ~ <List Char>
        ")
    };
    ctx.write().unwrap().morphisms.add_morphism(
        mt,
        {
            let ctx = ctx.clone();
            move |src_rt, σ| {
                let radix = σ.get( &laddertypes::TypeID::Var(ctx.read().unwrap().get_var_typeid("Radix").unwrap()) );
                let src_digits = src_rt.descend(Context::parse(&ctx, "
                     <PosInt Radix BigEndian>
                     ~ <Seq <Digit Radix>>
                     ~ <List <Digit Radix>~Char >
                    ").apply_substitution(&|k|σ.get(k).cloned()).clone()
                ).expect("cant descend")
                    .get_port::< dyn ListView<char> >().unwrap();

                let rev_port = src_digits.reverse();
                src_rt.attach_leaf_to(
                    Context::parse(&ctx, "
                            < PosInt Radix LittleEndian >
                            ~ < Seq <Digit Radix> >
                            ~ < List <Digit Radix>~Char >
                    ").apply_substitution(&|k| σ.get(k).cloned()).clone(),
                    rev_port
                );
            }
        }
    );






    let morphism_type = MorphismType {
        src_type: Context::parse(&ctx, "ℕ ~ <PosInt Radix LittleEndian> ~ <Seq <Digit Radix>~ℤ_2^64~machine.UInt64>"),
        dst_type: Context::parse(&ctx, "ℕ ~ <PosInt Radix BigEndian> ~ <Seq <Digit Radix>~ℤ_2^64~machine.UInt64>")
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
                 .view_seq::< u64 >();

                src_rt.attach_leaf_to(Context::parse(&ctx, "
                          <PosInt Radix BigEndian>
                        ~ <Seq <Digit Radix> ~ ℤ_2^64 ~ machine.UInt64>
                    ").apply_substitution(&|k|σ.get(k).cloned()).clone(),                        
                    src_digits.reverse()
                );
            }
        }
    );

    let morphism_type =
        MorphismType {
            src_type: Context::parse(&ctx, "
                  ℕ
                ~ <PosInt Radix LittleEndian>
                ~ <Seq <Digit Radix>>
                ~ <List <Digit Radix>>
                ~ <List ℤ_2^64>
                ~ <List machine.UInt64>
            "),
            dst_type: Context::parse(&ctx, "
                  ℕ
                ~ <PosInt Radix BigEndian>
                ~ <Seq <Digit Radix>>
                ~ <List <Digit Radix>>
                ~ <List ℤ_2^64>
                ~ <List machine.UInt64>
            ")
        };
    ctx.write().unwrap().morphisms.add_morphism(
        morphism_type, {
            let ctx = ctx.clone();
            move |src_rt, σ|
            {
                let src_digits = src_rt.descend(
                        Context::parse(&ctx, "
                              <PosInt Radix LittleEndian>
                            ~ <Seq <Digit Radix>>
                            ~ <List <Digit Radix>~ℤ_2^64~machine.UInt64 >
                        ").apply_substitution(&|k|σ.get(k).cloned()).clone()
                    )
                    .expect("cant descend")
                    .view_list::<u64>();

                src_rt.attach_leaf_to(
                    Context::parse(&ctx, "
                              <PosInt Radix BigEndian>
                            ~ <Seq <Digit Radix>>
                            ~ <List <Digit Radix>~ℤ_2^64~machine.UInt64 >
                    ").apply_substitution(&|k|σ.get(k).cloned()).clone(),
                    src_digits.reverse()
                );
            }
        }
    );


    let mt = MorphismType {
        src_type: Context::parse(&ctx, "
              ℕ 
            ~ <PosInt Radix LittleEndian>
            ~ <Seq <Digit Radix>>
            ~ <List <Digit Radix>>
            ~ <List Char>
        "),
        dst_type: Context::parse(&ctx, "
              ℕ
            ~ <PosInt Radix BigEndian>
            ~ <Seq <Digit Radix>>
            ~ <List <Digit Radix>>
            ~ <List Char>
        ")
    };

    ctx.write().unwrap().morphisms.add_morphism(
        mt,
        {
            let ctx = ctx.clone();
            move |src_rt, σ| {
                let src_digits = src_rt.descend(Context::parse(&ctx, "
                       <PosInt Radix LittleEndian>
                     ~ <Seq <Digit Radix>>
                     ~ <List <Digit Radix>~Char >
                    ").apply_substitution(&|k|σ.get(k).cloned()).clone()
                ).expect("cant descend")
                    .view_list::<char>();

                src_rt.attach_leaf_to(
                    Context::parse(&ctx, "
                              < PosInt Radix BigEndian >
                            ~ < Seq <Digit Radix> >
                            ~ < List <Digit Radix>~Char >
                    ").apply_substitution(&|k| σ.get(k).cloned()).clone(),
                    src_digits.reverse()
                );
            }
        }
    );
/*
    let mt = MorphismType {
        src_type: Context::parse(&ctx, "
              ℕ 
            ~ <PosInt Radix BigEndian>
            ~ <Seq <Digit Radix>>
            ~ <List <Digit Radix>>
            ~ <List Char>
        "),
        dst_type: Context::parse(&ctx, "
              ℕ
            ~ <PosInt Radix LittleEndian>
            ~ <Seq <Digit Radix>>
            ~ <List <Digit Radix>>
            ~ <List Char>
        ")
    };

    ctx.write().unwrap().morphisms.add_morphism(
        mt,
        {
            let ctx = ctx.clone();
            move |src_rt, σ| {    
                let src_digits = src_rt.descend(Context::parse(&ctx, "
                       <PosInt Radix BigEndian>
                     ~ <Seq <Digit Radix>>
                     ~ <List <Digit Radix>~Char >
                    ").apply_substitution(&|k|σ.get(k).cloned()).clone()
                ).expect("cant descend")
                    .view_list::<char>();

                src_rt.attach_leaf_to(
                    Context::parse(&ctx, "
                              < PosInt Radix LittleEndian >
                            ~ < Seq <Digit Radix> >
                            ~ < List <Digit Radix>~Char >
                    ").apply_substitution(&|k| σ.get(k).cloned()).clone(),
                    src_digits.reverse()
                );
            }
        }
    );
*/

    let morphism_type = MorphismType {
        src_type: Context::parse(&ctx, "
              ℕ
            ~ <PosInt SrcRadix LittleEndian>
            ~ <Seq   <Digit SrcRadix>>
            ~ <List  <Digit SrcRadix>
                   ~ ℤ_2^64
                   ~ machine.UInt64>
        "),
        dst_type: Context::parse(&ctx, "
              ℕ
            ~ <PosInt DstRadix LittleEndian>
            ~ <Seq   <Digit DstRadix>>
            ~ <List  <Digit DstRadix>
                   ~ ℤ_2^64
                   ~ machine.UInt64>
        ")
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
}

