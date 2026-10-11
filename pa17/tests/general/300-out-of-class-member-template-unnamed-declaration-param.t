template<class T>
struct value {
  template<int I>
  int get() {
    return I;
  }
};

template<class T>
struct owner {
  value<T> v;

  template<int>
  int read();
};

template<class T>
template<int I>
int owner<T>::read() {
  return v.template get<I>();
}

int main() {
  owner<int> instance;
  return instance.read<3>() == 3 ? 0 : 1;
}
