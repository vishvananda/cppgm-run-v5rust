#pragma once

#include <cstddef>
#include <string>

#include "recog_token_buffer.h"

namespace parser_trace {

bool enabled(const char * category);

void note(const char * category,
          const IRecogTokenSequence & tokens,
          std::size_t pos,
          const std::string & message);
void note(const char * category,
          const std::string & location,
          const std::string & message);

void push_use_location(const std::string & location);
void pop_use_location();
std::string current_use_location();
std::string current_order_use_location();
bool use_location_suppressed();

class ScopedOrderUseLocation
{
public:
  explicit ScopedOrderUseLocation(std::string location);
  ~ScopedOrderUseLocation();
  ScopedOrderUseLocation(const ScopedOrderUseLocation &) = delete;
  ScopedOrderUseLocation & operator=(const ScopedOrderUseLocation &) = delete;

private:
  std::string location_;
  bool active_;
};

std::string dump();
void append_to_error(std::string & error);

}  // namespace parser_trace
