template<class T, class U, class = void>
inline const bool v = false;

template<class T>
inline const bool v<T, T> = true;
