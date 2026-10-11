#pragma once

#include <cstddef>
#include <deque>
#include <string>
#include <vector>

#include "recog_token.h"
#include "source_location.h"

struct IRecogTokenSequence
{
  virtual ~IRecogTokenSequence() {}
  virtual const RecogToken & peek(std::size_t index) const = 0;
  virtual const RecogToken & operator[](std::size_t index) const = 0;
  virtual std::size_t size() const = 0;
  virtual const RecogToken & back() const = 0;
  virtual std::vector<RecogToken> slice(std::size_t start, std::size_t end) const = 0;
  virtual std::string span_text(std::size_t start, std::size_t end) const = 0;
  virtual const SourceLocationTable * source_locations() const { return nullptr; }
  virtual const std::string & primary_source_file() const
  {
    static const std::string empty;
    return empty;
  }
};

struct RecogTokenBuffer : IRecogTokenSequence
{
  explicit RecogTokenBuffer(IRecogTokenSource & source,
                            const std::string & debug_label = std::string(),
                            const SourceLocationTable * locations = nullptr);
  ~RecogTokenBuffer() override;

  const RecogToken & peek(std::size_t index) const override;
  const RecogToken & operator[](std::size_t index) const override;
  std::size_t size() const override;
  const RecogToken & back() const override;
  std::vector<RecogToken> slice(std::size_t start, std::size_t end) const override;
  std::string span_text(std::size_t start, std::size_t end) const override;
  const SourceLocationTable * source_locations() const override;
  const std::string & primary_source_file() const override;

#ifdef CPPGM_DEBUG_RECOG_BUFFER
  void debug_note_peek(std::size_t pos, std::size_t offset) const;
#endif

private:
  void ensure(std::size_t index) const;
  void append_token_batch(std::vector<RecogToken> & batch) const;
  const RecogToken & last_pulled() const;
  void materialize_all() const;

  IRecogTokenSource & source;
  mutable std::deque<RecogToken> tokens;
  mutable bool finished;
  const SourceLocationTable * locations;
  std::string primary_source_file_;

#ifdef CPPGM_DEBUG_RECOG_BUFFER
  std::string debug_label;
  mutable std::size_t debug_max_materialized_tokens;
  mutable std::size_t debug_max_peek_offset;
  mutable std::size_t debug_max_buffered_ahead;
  mutable std::size_t debug_forced_full_materializations;
  mutable std::size_t debug_max_forced_materialized_tokens;
  mutable bool debug_used;
#endif
};
