// A class with a non-trivial copy constructor crosses a this-adjusting
// virtual thunk by invisible reference, as a parameter and as a result.
struct Value {
  long n;
  const void* self;
  Value(long v) : n(v), self(this) {}
  Value(const Value& other) : n(other.n), self(this) {}
};

struct First {
  virtual Value first(Value value) = 0;
  long first_tag;
};

struct Second {
  virtual Value second(Value value) = 0;
  long second_tag;
};

struct Both : First, Second {
  Value first(Value value);
  Value second(Value value);
};

Second* host_second();
int host_calls_student(Second* second);
Second* student_second();
