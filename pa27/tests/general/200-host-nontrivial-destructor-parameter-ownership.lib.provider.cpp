#include "200-host-nontrivial-destructor-parameter-ownership.helper.h"

namespace ownership {
int host_take(Tracked value) {
    // The caller's copy is still alive here, beside the caller's original.
    return value.self == &value && live == 2 ? value.id : -1;
}

int host_calls_student() {
    int status = 0;
    Tracked original(30);
    const int made_before = made;
    const int destroyed_before = destroyed;
    bool alive_after_return = student_take(original) == 30 && live == 2;
    if (!alive_after_return) status |= 1;
    if (live != 1 || made - made_before != 1 ||
        destroyed - destroyed_before != 1) status |= 2;
    student_take(Tracked(31));
    if (live != 1 || made - made_before != 2 ||
        destroyed - destroyed_before != 2) status |= 4;
    return status;
}
}
