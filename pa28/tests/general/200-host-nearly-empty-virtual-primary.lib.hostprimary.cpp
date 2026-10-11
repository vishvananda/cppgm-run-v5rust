#include "200-host-nearly-empty-virtual-primary.helper.h"

int LeftImpl::id() { return 10; }
Tagged::Tagged() : value(tag()) {}
int Tagged::tag() { return 2; }

long host_signature() { return signature(); }
int host_id(Interface& value) { return value.id(); }
int host_right_id(RightImpl& value) { return value.id(); }
int host_tag(Tag& value) { return value.tag(); }
MoreTagged* host_make()
{
  static MoreTagged more;
  return &more;
}
