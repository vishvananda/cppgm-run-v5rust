namespace n { typedef unsigned long size_type; }

template<class T, T... I> struct sequence {};
template<n::size_type... I> using index_sequence = sequence<n::size_type, I...>;

template<n::size_type... I>
int count(index_sequence<I...>) { return sizeof...(I); }

int main() {
  return count(sequence<n::size_type, 0>()) == 1 ? 0 : 1;
}
