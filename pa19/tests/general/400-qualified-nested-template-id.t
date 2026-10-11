// HHC-146
template<bool B>
struct IfImpl;

template<>
struct IfImpl<true> {
  template<class IfRes, class ElseRes>
  using Select = IfRes;
};

using X = typename IfImpl<true>::template Select<int, long>;
int main() { return 0; }
