typedef decltype(sizeof(0)) size_t;

template<size_t... I>
struct index_sequence {};

template<class T>
void probe(T) {
  [&]<size_t... I>(index_sequence<I...>) -> void {
    (void)sizeof...(I);
  }(index_sequence<0>{});
}

int main() {
  return 0;
}
