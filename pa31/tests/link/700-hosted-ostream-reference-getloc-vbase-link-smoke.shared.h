#pragma once

#include <iosfwd>
#include <locale>

struct HostedOstreamLocaleProbe
{
  std::ostream & os;
  std::locale archive_locale;

  explicit HostedOstreamLocaleProbe(std::ostream & os_);
  int touch();
};
