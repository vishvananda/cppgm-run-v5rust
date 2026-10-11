// VALIDATION: compile-pass

struct pa13_reentrant_override_stream
{
};

struct pa13_reentrant_override_entry
{
};

struct pa13_reentrant_override_base
{
  enum entry_kind
  {
    entry_info
  };

  virtual ~pa13_reentrant_override_base()
  {
  }

  virtual void start(pa13_reentrant_override_stream&,
                     pa13_reentrant_override_entry const&,
                     entry_kind) = 0;
};

template<class T>
struct pa13_reentrant_override_holder
{
  typename T::tag* p;
};

struct pa13_reentrant_override_derived : pa13_reentrant_override_base
{
  typedef int tag;

  void start(pa13_reentrant_override_stream&,
             pa13_reentrant_override_entry const&,
             entry_kind) override;

  pa13_reentrant_override_holder<pa13_reentrant_override_derived> member;
};

void pa13_reentrant_override_derived::start(pa13_reentrant_override_stream&,
                                            pa13_reentrant_override_entry const&,
                                            entry_kind)
{
}

int main()
{
  pa13_reentrant_override_derived d;
  (void)d;
  return 0;
}
