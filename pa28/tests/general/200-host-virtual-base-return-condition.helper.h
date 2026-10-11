struct HostFlagExtCtor {
  int value;
  HostFlagExtCtor();
  virtual ~HostFlagExtCtor();
  operator bool() const;
};

struct HostStreamExtCtor : virtual HostFlagExtCtor {
  HostStreamExtCtor();
  virtual ~HostStreamExtCtor();
};

struct HostStringExtCtor : HostStreamExtCtor {
  HostStringExtCtor();
  virtual ~HostStringExtCtor();
};

HostStreamExtCtor &host_getline_extctor(HostStreamExtCtor &stream);
