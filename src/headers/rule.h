#pragma once

// Rules are direct enforcers of object structure and further hinting upon it. 
// For example, PairRule explicitly defines HOW to handle a pair, and when it would happen. 
// This is the same for ParentRule, ChildRule, etc. 
// Where the user handles actual page classification. 

#include "state.h"
#include "weights.h"

#define UPDATE_SCORE_IF(condition, score, by) \
    if (condition)                            \
    {                                         \
        score += by;                          \
    }

class Rule
{
public:
    virtual float when(const State &current_state) = 0;
};

class PairRule : public Rule
{
public:
    float when(const State &current_state) override
    {
        std::shared_ptr<Object> prev_obj_ptr = current_state.previous_classified_obj();
        std::shared_ptr<Object> current_obj_ptr = current_state.current_considering_obj();

        bool both_are_pair = prev_obj_ptr->is_pair() && current_obj_ptr->is_pair();
        bool both_are_eachothers_pair = prev_obj_ptr->get_pair_rep().value() == current_obj_ptr->representitive;
        bool pair_order_enforced = prev_obj_ptr->is_first_in_pair() && current_obj_ptr->is_second_in_pair();

        float score = 0.0;
        UPDATE_SCORE_IF(both_are_pair, score, weights::pair::BOTH_ARE_PAIR_BONUS)
        UPDATE_SCORE_IF(both_are_eachothers_pair, score, weights::pair::BOTH_ARE_EACHOTHERS_PAIR)
        UPDATE_SCORE_IF(pair_order_enforced, score, weights::pair::PAIR_ORDER_ENFORCED)

        return score;
    }
};