// N3485 focus: [temp.expl.spec], [temp.deduct.decl].
// An explicit function-template specialization with explicit template
// arguments still selects the overload whose function parameter list matches
// the specialization declaration.

struct context;
struct io_context;

struct service {
  int value;
  service() : value(0) {}
};

template<class Service>
Service& use_service(context& e);

template<class Service>
Service& use_service(io_context& ioc);

struct context {
  context() { base_service_.value = 3; }

  template<class Service>
  friend Service& use_service(context& e);

private:
  service base_service_;
};

struct io_context : context {
  io_context() { impl_.value = 7; }

  template<class Service>
  friend Service& use_service(io_context& ioc);

private:
  service impl_;
};

template<class Service>
Service& use_service(context& e) {
  return e.base_service_;
}

template<class Service>
Service& use_service(io_context& ioc) {
  return ioc.impl_;
}

template<>
service& use_service<service>(io_context& ioc) {
  return ioc.impl_;
}

int main() {
  context c;
  return use_service<service>(c).value == 3 ? 0 : 1;
}
