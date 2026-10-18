// expect exit 0
// Distinct closure types select distinct member-template specializations;
// retained lambda bodies keep their lexical namespace and parameter scope.
namespace checks {
int effects;
struct chain {
  template<class F> chain step(F const& f) {
    effects += f(3);
    return chain();
  }
};
inline int run() {
  chain().step([](int x) { return x + 1; })
         .step([](int x) -> int { return x + 2; });
  return effects;
}
}
int main() { return checks::run() == 9 ? 0 : 1; }
