struct SemanticContext;

namespace host_templates {
template<class T>
struct Allocator;

template<class T, class Alloc = Allocator<T> >
struct Vector;
}

namespace semantic_model {
struct Scope;
struct ClassInfo;
struct FunctionBinding;
}

namespace semantic_conversion {
struct ExprInfo;
enum class ConversionRank;
}

namespace semantic_overload {
struct ConstructorSelectionOptions;

semantic_model::FunctionBinding * select_constructor_from_exprs(
    SemanticContext & ctx,
    semantic_model::Scope & scope,
    semantic_model::ClassInfo & info,
    const host_templates::Vector<semantic_conversion::ExprInfo> & source_args,
    host_templates::Vector<semantic_conversion::ExprInfo> & args_out,
    host_templates::Vector<semantic_conversion::ConversionRank> * ranks_out,
    const ConstructorSelectionOptions & options);
}
