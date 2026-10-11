// A by-value parameter whose class has a non-trivial destructor belongs to
// the caller, which destroys it at the end of the full-expression containing
// the call; the callee never destroys it (Itanium C++ ABI 3.1.2.3).
namespace ownership {
extern int live, made, destroyed;

struct Tracked {
    int id;
    const void* self;
    Tracked(int v);
    Tracked(const Tracked& other);
    ~Tracked();
};

int host_take(Tracked value);
int student_take(Tracked value);
int host_calls_student();
}
