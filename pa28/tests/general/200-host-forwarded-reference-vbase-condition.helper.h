struct HostForwardFlag {
  int value;

  HostForwardFlag();
  virtual ~HostForwardFlag();
  operator bool() const;
};

struct HostForwardStream : virtual HostForwardFlag {
  HostForwardStream();
  virtual ~HostForwardStream();
};

struct HostForwardDerived : HostForwardStream {
  long tail[20];

  HostForwardDerived();
  virtual ~HostForwardDerived();
};

HostForwardDerived &host_forward_derived();
HostForwardStream &host_forward_stream(HostForwardStream &stream);
