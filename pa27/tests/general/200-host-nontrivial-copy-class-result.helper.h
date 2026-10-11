namespace result_abi {
struct Base { int n; };

// Non-trivial copy constructor in a small derived class.
struct DerivedCopy : Base {
    const void* self;
    DerivedCopy(int v) : self(this) { n = v; }
    DerivedCopy(const DerivedCopy& other) : Base(other), self(this) {}
};

// Non-trivial copy constructor beside a trivial move constructor.
struct CopyTrivialMove {
    int n;
    CopyTrivialMove(int v) : n(v) {}
    CopyTrivialMove(const CopyTrivialMove& other) : n(other.n + 1) {}
    CopyTrivialMove(CopyTrivialMove&&) = default;
};

DerivedCopy host_derived(int);
CopyTrivialMove host_mixed(const CopyTrivialMove&);
DerivedCopy student_derived(int);
CopyTrivialMove student_mixed(const CopyTrivialMove&);
int host_calls_student();
}
