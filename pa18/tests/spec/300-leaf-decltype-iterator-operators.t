// VALIDATION: compile-pass
// A dependent decltype condition must order iterator comparison overloads and
// use an overloaded dereference when selecting a class partial specialization.

template<bool> struct enable_if {};
template<> struct enable_if<true> { typedef void type; };

template<class> struct valid { static const bool value = true; };
template<class> constexpr bool valid_element() { return true; }
template<class T> T&& declval();

template<class T> struct iterator { int& operator*() const; };
template<class T> bool operator!=(const iterator<T>&, const iterator<T>&);
template<class T, class U>
bool operator!=(const iterator<T>&, const iterator<U>&);

struct range { iterator<int> begin(); iterator<int> end(); };
template<class T> auto begin(T& value) -> decltype(value.begin());
template<class T> auto end(T& value) -> decltype(value.end());

template<class T, class = void>
struct is_range { static const bool value = false; };

template<class T>
struct is_range<T, typename enable_if<
    valid<decltype(begin(declval<T&>()) != end(declval<T&>()))>::value &&
    valid_element<decltype(*begin(declval<T&>()))>()>::type>
{
  static const bool value = true;
};

static_assert(is_range<range>::value, "");

int main() { return 0; }
