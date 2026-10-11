#include "200-host-nontrivial-sixteen-byte-class-result.helper.h"

namespace result_abi {
Value::Value(long long x) : first(x), second(42) {}
Value::~Value() {}
Value host_make(long long x) { return Value(x); }
int host_check_student() {
    Value value = student_make(73);
    return value.first != 73 || value.second != 42;
}
}
