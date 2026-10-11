template<class T> T* value();
template<class T> struct trait { static const bool value = noexcept(::value<T>()->~T()); };
static_assert(trait<int>::value, "");
