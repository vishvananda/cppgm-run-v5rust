#include "200-host-nontrivial-copy-class-parameter.helper.h"

namespace call_abi {
int host_take(DerivedCopy a, CopyTrivialMove b, TrivialCopyMove c,
    NoCopy d, int tail) {
    int status = 0;
    if (a.n != 11 || a.self != &a) status |= 1;
    if (b.n != 12 || b.self != &b) status |= 2;
    if (c.n != 13 || c.self != &c) status |= 4;
    if (d.n != 14 || tail != 15) status |= 8;
    return status;
}

int host_calls_student() {
    DerivedCopy a(21);
    CopyTrivialMove b(22);
    return student_take(a, b, TrivialCopyMove(23), {24}, 25);
}
}
