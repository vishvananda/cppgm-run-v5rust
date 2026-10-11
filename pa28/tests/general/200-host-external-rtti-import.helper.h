struct HostBase {
  virtual ~HostBase();
};

struct HostDerived : HostBase {
  virtual int value() const;
};

HostBase * make_host_base();
