#pragma once

namespace header_friend_alias {

enum error_type
{
  error_bad = 7
};

class Error
{
  using local_error_type = error_type;

  friend int throw_like(local_error_type code, const char * text)
  {
    return code + text[0];
  }
};

inline int throw_like(error_type code, const char * text);

}
