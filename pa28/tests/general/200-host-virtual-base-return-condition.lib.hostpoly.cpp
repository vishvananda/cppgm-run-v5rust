#include "200-host-virtual-base-return-condition.helper.h"

HostFlagExtCtor::HostFlagExtCtor() : value(0) {}
HostFlagExtCtor::~HostFlagExtCtor() {}
HostStreamExtCtor::HostStreamExtCtor() {}
HostStreamExtCtor::~HostStreamExtCtor() {}
HostStringExtCtor::HostStringExtCtor() {}
HostStringExtCtor::~HostStringExtCtor() {}

HostFlagExtCtor::operator bool() const
{
  return value == 12345;
}

HostStreamExtCtor &host_getline_extctor(HostStreamExtCtor &stream)
{
  stream.value = 12345;
  return stream;
}
