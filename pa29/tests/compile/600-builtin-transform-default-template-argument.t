namespace cpp_decl {
struct TemplateIdSyntax {};
}

namespace std {
namespace __1 {

template<bool B>
struct integral_constant {
  static const bool value = B;
};

template<bool B>
using bool_constant = integral_constant<B>;

template<class T>
struct allocator {};

template<class T, class Alloc = allocator<T> >
struct vector {};

template<class T>
struct is_void : bool_constant<__is_same(__remove_cv(T), void)> {};

}
}

int main() {
  return std::__1::is_void<std::__1::vector<cpp_decl::TemplateIdSyntax> >::value ? 1 : 0;
}
