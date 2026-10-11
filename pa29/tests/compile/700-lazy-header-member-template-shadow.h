#pragma once

namespace lazy_header_member_template_shadow {

struct header {
  template<class CharT>
  void *name()
  {
    return 0;
  }
};

template<class T>
struct manager {
  template<class CharT>
  CharT *get(header *hdr, const CharT *name)
  {
    (void)name;
    CharT *name_ptr = static_cast<CharT *>(hdr->template name<CharT>());
    return name_ptr;
  }
};

}
