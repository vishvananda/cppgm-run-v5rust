#include "200-host-forwarded-reference-vbase-condition.helper.h"

HostForwardFlag::HostForwardFlag() : value(0)
{
}

HostForwardFlag::~HostForwardFlag()
{
}

HostForwardFlag::operator bool() const
{
  return value == 12345;
}

HostForwardStream::HostForwardStream()
{
}

HostForwardStream::~HostForwardStream()
{
}

HostForwardDerived::HostForwardDerived()
{
  for(int i = 0; i < 20; ++i) {
    tail[i] = 0;
  }
}

HostForwardDerived::~HostForwardDerived()
{
}

static HostForwardDerived g_stream;

HostForwardDerived &host_forward_derived()
{
  return g_stream;
}

HostForwardStream &host_forward_stream(HostForwardStream &stream)
{
  stream.value = 12345;
  return stream;
}
