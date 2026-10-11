template<class T> struct Mask {};
template<class T> struct Expr {};

template<class T>
struct Box {
  Expr<T> operator[](const Box<bool>&) const;
  Mask<T> operator[](const Box<bool>&);
  Expr<T> operator[](Box<bool>&&) const;
  Mask<T> operator[](Box<bool>&&);
};

template<class T>
Expr<T> Box<T>::operator[](const Box<bool>&) const { return Expr<T>(); }

template<class T>
Mask<T> Box<T>::operator[](const Box<bool>&) { return Mask<T>(); }

template<class T>
Expr<T> Box<T>::operator[](Box<bool>&&) const { return Expr<T>(); }

template<class T>
Mask<T> Box<T>::operator[](Box<bool>&&) { return Mask<T>(); }

int main() {
  Box<int> b;
  return 0;
}
