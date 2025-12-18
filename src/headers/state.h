#pragma once

// The state is the context that we have built up at a point of time N over the "master" thread.
// Where the master thread is the thread that intializes worker threads with their own corresponding MuPDF contexts.
// Prior to any classification, each thread does exactly that and "rests" until the statemachine has theorized over a page
// Then, the master thread will copy the current State and provide it to a worker thread (which already has its PDF context)
// As the master thread continues to gather information from worker threads returning data,
// On the event of a worker thread failing to classify a page as whatever type, it will report back to the state machine.

#include <unordered_map>
#include <vector>
#include <format>
#include "object.h"
#include "reflected_objects.h"
#include "generated_page_types.h"

using PageNum = uint32_t;

class State
{
    const std::unordered_map<KnownObject, std::shared_ptr<Object>> m_known_objs;
    std::vector<KnownObject> m_current_classified;
    KnownObject m_considering;
    PageNum m_current_page;

public:
    const std::map<KnownObject, std::shared_ptr<Object>> &window(int start, int end) const;

    const KnownObject &previous_classified_ident() const;

    std::shared_ptr<Object> previous_classified_obj() const;

    std::shared_ptr<Object> current_considering_obj() const;

    std::shared_ptr<Object> lookup_ident(KnownObject ident) const;

    State();
};
