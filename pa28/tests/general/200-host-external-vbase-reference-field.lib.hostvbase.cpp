#include "200-host-external-vbase-reference-field.helper.h"

HostVBase::HostVBase()
  : value(77)
{
}

HostVBase::~HostVBase()
{
}

HostStream::HostStream()
{
}

HostStream::~HostStream()
{
}

HostStringStream::HostStringStream()
{
}

HostStringStream::~HostStringStream()
{
}

static HostStringStream g_stream;

HostStream & host_stream()
{
  return g_stream;
}
