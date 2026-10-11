// N3485 8.5.3p5/4.10p3: IB& bound to an SS lvalue, where IB is a base of SS's
// indirect virtual base BI, refers to the same subobject as the pointer
// conversion (std::ios_base& from std::ostringstream); expect exit 0
struct IB { int f; };
struct BI : IB { int x; };
struct OS : virtual BI { int o; };
struct SS : OS { int s; };
static IB* addr(IB& b) { return &b; }
int main() {
  SS s;
  IB* p = static_cast<IB*>(static_cast<BI*>(&s));
  return addr(s) == p ? 0 : 1;
}
