namespace std {
inline namespace __1 {

struct _ClassicAlgPolicy {};

template <class P> struct _IterOps {
  template <class I> using __difference_type = long;
};

template <class P, class I, class O>
int __copy_n(I, typename _IterOps<P>::template __difference_type<I>, O) {
  return 7;
}

}
}

int main() {
  return std::__copy_n<std::_ClassicAlgPolicy>((const char*)0, 1, (char*)0);
}
