namespace call_abi {
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
    const void* self;
    CopyTrivialMove(int v) : n(v), self(this) {}
    CopyTrivialMove(const CopyTrivialMove& other) : n(other.n), self(this) {}
    CopyTrivialMove(CopyTrivialMove&&) = default;
};

// Trivial copy constructor beside a non-trivial move constructor.
struct TrivialCopyMove {
    int n;
    const void* self;
    TrivialCopyMove(int v) : n(v), self(this) {}
    TrivialCopyMove(const TrivialCopyMove&) = default;
    TrivialCopyMove(TrivialCopyMove&& other) : n(other.n), self(this) {}
};

// Every copy and move constructor deleted.
struct NoCopy {
    int n;
    NoCopy(int v) : n(v) {}
    NoCopy(const NoCopy&) = delete;
};

int host_take(DerivedCopy a, CopyTrivialMove b, TrivialCopyMove c,
    NoCopy d, int tail);
int student_take(DerivedCopy a, CopyTrivialMove b, TrivialCopyMove c,
    NoCopy d, int tail);
int host_calls_student();
}
