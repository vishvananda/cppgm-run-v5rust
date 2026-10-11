struct Scope
{
  int value;
};

namespace api
{
  struct Services
  {
    int value;
  };

  struct Handle
  {
    Handle()
      : scope(0)
    {
    }

    explicit Handle(Scope & scope_in)
      : scope(&scope_in)
    {
    }

    Scope * scope;
  };
}

template<class T>
struct Box
{
  T * ptr;
};

struct Type
{
  int value;
};

namespace dep
{
  bool resolve(api::Services & services,
               api::Handle scope,
               const Box<Type> & type,
               Box<Type> & out);
}

bool wrapper(api::Services & services,
             Scope & raw_scope,
             const Box<Type> & type,
             Box<Type> & out)
{
  return dep::resolve(services, api::Handle(raw_scope), type, out);
}
