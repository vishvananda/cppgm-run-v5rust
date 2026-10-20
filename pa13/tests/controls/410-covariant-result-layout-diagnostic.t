// N3485 10.3p7: checking a primary covariant slot demands the result layout
// at the completing class, including its ordinary size-limit diagnostic.
// expect reject
struct Base {};
struct Result : Base { char huge[1152921504606846976]; };
struct Primary { virtual Base* get(); };
struct Derived : Primary { Result* get(); };
