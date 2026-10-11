// VALIDATION: compile-pass
// A later using-directive cannot change an earlier member typedef's lookup.

namespace N {

template<bool>
struct choose
{
  typedef int type;
};

namespace late {
struct terminal {};
terminal const lower = terminal();
}

template<class T>
struct make;

template<class T>
struct make<T*>
{
  static bool const lower = sizeof(T);
  typedef typename choose<lower || false>::type result_type;
};

using namespace late;

}

static_assert(sizeof(N::make<int*>::result_type) == sizeof(int), "");

int main() {}
