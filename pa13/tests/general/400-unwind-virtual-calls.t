// lowir.md unwind=no on call signatures: a call signature states unwind=no
// exactly when the callee's declared type is non-throwing: a virtual call of a
// noexcept function does, and so does a delete-expression's virtual call of the
// deleting destructor when the destructor is non-throwing (12.4p3).
struct Quiet
{
  virtual int fast() const noexcept { return 1; }
  virtual int slow() const { return 2; }
  virtual ~Quiet() {}
};
struct Loud
{
  virtual ~Loud() noexcept(false) {}
};

int call(const Quiet* q) { return q->fast() + q->slow(); }
void drop_quiet(Quiet* q) { delete q; }
void drop_loud(Loud* l) { delete l; }

int main()
{
  Quiet* q = new Quiet;
  int result = call(q);
  drop_quiet(q);
  drop_loud(new Loud);
  return result == 3 ? 0 : 1;
}
