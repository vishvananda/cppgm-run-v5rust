template<class T, unsigned long = 0>
inline const bool v = false;

template<class T>
inline const bool v<T, sizeof(T) * 0> = true;
