// N3485 focus: 5.3.1 [expr.unary.op], pointer conversion before indirection.
struct value;
struct convertible { operator value *() const; };
value & dereference(convertible input) { return *input; }
