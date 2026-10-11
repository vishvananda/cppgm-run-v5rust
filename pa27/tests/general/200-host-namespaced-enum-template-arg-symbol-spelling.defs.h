struct SemanticContext {};

namespace host_templates {
template<class T>
struct Allocator {};

template<class T, class Alloc>
struct Vector {};
}

namespace semantic_model {
struct Scope {};
struct ClassInfo {};
struct FunctionBinding {};
}

namespace semantic_conversion {
struct ExprInfo {};
enum class ConversionRank { exact };
}

namespace semantic_overload {
struct ConstructorSelectionOptions {};
}
