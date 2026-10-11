#include "200-host-nontrivial-copy-class-result.helper.h"

namespace result_abi {
DerivedCopy host_derived(int v) { return DerivedCopy(v); }
CopyTrivialMove host_mixed(const CopyTrivialMove& source) { return source; }

int host_calls_student() {
    int status = 0;
    DerivedCopy a = student_derived(31);
    if (a.n != 31 || a.self != &a) status |= 4;
    CopyTrivialMove source(40);
    CopyTrivialMove b = student_mixed(source);
    if (b.n != 41) status |= 8;
    return status;
}
}
