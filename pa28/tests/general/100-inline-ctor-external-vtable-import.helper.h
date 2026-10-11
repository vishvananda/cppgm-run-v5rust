struct HostPoly {
  HostPoly() {}
  virtual int f() const;
  virtual ~HostPoly();
};

inline int use_host_poly()
{
  HostPoly p;
  return p.f();
}
