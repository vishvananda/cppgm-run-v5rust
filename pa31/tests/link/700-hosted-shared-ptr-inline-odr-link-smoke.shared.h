#pragma once

#include <memory>
#include <string>

inline int shared_ptr_inline_probe()
{
  std::shared_ptr<std::string> value = std::make_shared<std::string>("ok");
  std::shared_ptr<std::string> copy = value;
  return static_cast<int>(copy->size());
}
