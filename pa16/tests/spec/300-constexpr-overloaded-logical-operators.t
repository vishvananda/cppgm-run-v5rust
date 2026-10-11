// N3485 focus: 5.19 [expr.const], 13.5 [over.oper]
struct value {
  int data;
  constexpr value(int input) : data(input) {}
  constexpr explicit operator bool() const { return data != 0; }
};

constexpr value operator||(bool lhs, value rhs) { return lhs ? value(1) : rhs; }
constexpr value operator||(value lhs, bool rhs) { return lhs ? value(1) : value(rhs); }
constexpr value operator&&(bool lhs, value rhs) { return lhs ? rhs : value(0); }
constexpr value operator&&(value lhs, bool rhs) { return lhs ? value(rhs) : value(0); }

static_assert((false || value(0) || true).data == 1, "overloaded logical or");
static_assert((true && value(1) && false).data == 0, "overloaded logical and");
static_assert(static_cast<bool>(value(1)), "explicit bool conversion");

int main() {}
