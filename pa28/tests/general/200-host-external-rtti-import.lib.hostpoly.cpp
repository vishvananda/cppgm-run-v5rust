#include "200-host-external-rtti-import.helper.h"

HostBase::~HostBase()
{
}

int HostDerived::value() const
{
  return 9;
}

HostBase * make_host_base()
{
  static HostDerived instance;
  return &instance;
}
