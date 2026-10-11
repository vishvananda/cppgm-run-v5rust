#include "700-lazy-header-member-template-shadow.h"

int main()
{
  lazy_header_member_template_shadow::header hdr;
  lazy_header_member_template_shadow::manager<int> manager;
  return manager.get(&hdr, "x") == 0 ? 0 : 1;
}
