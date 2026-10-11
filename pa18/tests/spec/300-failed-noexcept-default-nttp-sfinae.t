template<class T> T&& value();
void swap(int&, int&) noexcept;
template<bool B> struct constant { static const bool value = B; };
template<class T, bool B = noexcept(swap(value<T&>(), value<T&>()))> constant<B> probe(int);
template<class> constant<false> probe(...);
static_assert(!decltype(probe<const int>(0))::value, "");
