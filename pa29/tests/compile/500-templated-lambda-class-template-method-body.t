typedef decltype(sizeof(0)) size_t;

template<size_t... I>
struct index_sequence {};

template<class F>
class call_once_param {
public:
  explicit call_once_param(F& f);
  void operator()();

private:
  F* f_;
};

template<class F>
call_once_param<F>::call_once_param(F& f) : f_(&f) {}

template<class F>
void call_once_param<F>::operator()() {
  [&]<size_t... I>(index_sequence<I...>) -> void {
    (void)sizeof...(I);
  }(index_sequence<0>{});
}

int main() {
  return 0;
}
