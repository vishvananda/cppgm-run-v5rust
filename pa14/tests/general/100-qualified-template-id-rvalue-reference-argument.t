// A namespace-qualified template-id followed by an rvalue-reference abstract
// declarator must remain a structured type template argument.
namespace library {
template<class T>
struct box {};
}

template<class T>
struct holder {};

holder<library::box<int>&&> make_holder();

int main()
{
  return 0;
}
