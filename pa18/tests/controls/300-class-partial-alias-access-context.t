// expect exit 0
template<class...> using erased = void;
template<class T> using member = typename T::type;
template<class T> using nested = member<T>;

template<class T, class = void>
struct Friend { static const int value = 1; };
template<class T, class = void>
struct Stranger { static const int value = 3; };

class Hidden {
  typedef int type;
  template<class, class> friend struct Friend;
};
struct Public { typedef int type; };
struct Protected { protected: typedef int type; };

template<class T>
struct Friend<T, erased<nested<T>>> { static const int value = 2; };
template<class T>
struct Stranger<T, erased<nested<T>>> { static const int value = 4; };

static_assert(Stranger<Hidden>::value == 3, "private alias fails substitution");
static_assert(Stranger<Protected>::value == 3, "protected alias fails substitution");
static_assert(Stranger<Public>::value == 4, "public alias is viable");
static_assert(Friend<Hidden>::value == 2, "primary friendship reaches its partial");

int main() { return 0; }
