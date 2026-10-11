#include <type_traits>

template<class T>
struct parser_like {
  ~parser_like() = default;
};

static_assert(
    std::is_nothrow_destructible<parser_like<int> >::value,
    "defaulted destructor class-template specialization is nothrow destructible");

int main()
{
  return 0;
}
