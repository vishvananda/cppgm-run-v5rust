struct HostVBase {
  int value;

  HostVBase();
  virtual ~HostVBase();

  int get() const
  {
    return value;
  }
};

struct HostStream : virtual HostVBase {
  HostStream();
  virtual ~HostStream();
};

struct HostStringStream : HostStream {
  int padding[16];

  HostStringStream();
  virtual ~HostStringStream();
};

HostStream & host_stream();

struct Holder {
  HostStream & stream;

  explicit Holder(HostStream & in)
    : stream(in)
  {
  }

  int read() const
  {
    return stream.HostVBase::get();
  }
};
