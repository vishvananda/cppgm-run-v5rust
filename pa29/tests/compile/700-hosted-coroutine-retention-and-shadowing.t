template<class T> T unused(T t){
  auto a = co_await (T)t;
  co_yield {t};
  co_return {a};
}
int co_await(int n){return n+1;}
int main(){ int co_yield = 1; int co_return = 2; return co_await(co_yield) - co_return; }

// Internal operations with fixed arguments also wait for demand.
template<class T> void fixed_operands(T t) { co_yield 1; }
// Parameter declarations keep all three names ordinary during body parsing.
template<class T> T declared(T co_await, T co_yield, T co_return) {
  co_return = co_await + co_yield;
  return co_return;
}
int instantiate_declared() { return declared(1, 2, 3); }
