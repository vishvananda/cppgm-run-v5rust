// N3485 focus: 5.2.3 [expr.type.conv], 5.19 [expr.const]
template<class T>
constexpr T value_init() { return T(); }

static_assert(value_init<char>() == 0, "value initialization");

int main() {}
