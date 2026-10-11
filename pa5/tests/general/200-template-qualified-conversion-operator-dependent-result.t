template<class T> struct Val {};

template<class T>
struct Box {
  using value_type = T;
  operator Val<value_type>() const;
};

template<class T>
Box<T>::operator Val<Box::value_type>() const {}
