// The caller owns a by-address class parameter on every path: it destroys
// the parameter when the callee throws and when a this-adjusting thunk
// forwarded the call (Itanium C++ ABI 3.1.2.3).
extern int live, made, destroyed;

struct Tracked {
  int id;
  const void* self;
  Tracked(int v);
  Tracked(const Tracked& other);
  ~Tracked();
};

struct First {
  virtual int first(Tracked value) = 0;
  long first_tag;
};

struct Second {
  virtual int second(Tracked value) = 0;
  long second_tag;
};

int host_throw(Tracked value);
int student_throw(Tracked value);
Second* host_second();
Second* student_second();
int host_calls_student();
