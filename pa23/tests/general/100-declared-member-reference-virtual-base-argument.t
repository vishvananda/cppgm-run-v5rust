struct IOS {
  int flags;
  IOS() : flags(0) {}
};

struct OStream : virtual IOS {
  OStream() {}
};

struct Holder {
  OStream * stream;

  Holder(OStream & os) : stream(&os) {}

  OStream & ref() const
  {
    return *stream;
  }
};

struct Log {
  void set_stream(OStream & os);
};

int main()
{
  OStream os;
  Holder holder(os);
  Log log;
  log.set_stream(holder.ref());
  return 0;
}
