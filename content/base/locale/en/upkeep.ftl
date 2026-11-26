## Upkeep made visible (DEC-114): faults at spots on a neighbourhood's
## streets, the repair queue, and casework on a fault.

## The fault kinds (`faults.jsonc` `text`): a name to use mid-sentence, and
## what is wrong, as a resident or the street shows it.
fault-streetlight-name = the street light
fault-streetlight-what = A street light on the streets of { $hood } is out, and the corner is dark after six.
fault-drain-name = the blocked drain
fault-drain-what = A drain on the streets of { $hood } is blocked, and the gutter floods whenever it rains.
fault-pothole-name = the pothole
fault-pothole-what = There is a pothole on the streets of { $hood } deep enough to take a wheel off.
fault-shelter-name = the bus shelter
fault-shelter-what = The bus shelter in { $hood } has its glass out and the stop is taped off.
fault-swing-name = the broken swing
fault-swing-what = A swing on the green in { $hood } is broken and the play area is taped off.

## Events (the overview, "Why?").
event-fault-opened = In { $hood }, { $fault } needs repair.
event-fault-expedited = You had { $fault } in { $hood } moved up the repair queue.
event-fault-repairing = Work began on { $fault } in { $hood }, paid from the town's budget for it.
event-fault-repairing-ahead = Work began on { $fault } in { $hood }, ahead of { $other } in { $other_hood }.
event-fault-delayed = In { $hood }, the repair of { $fault } was put back: the money went to { $other } in { $other_hood }, moved up the queue.
event-fault-fixed = In { $hood }, { $fault } was repaired.

## The ask a resident brings about a fault (asks.jsonc `ask.fault`).
ask-fault = Get { $fault } repaired
ask-fault-detail = { $history ->
        [kept] { $asker } ({ $asker_role }) remembers you helped before.{ " " }
        [broken] { $asker } ({ $asker_role }) has not forgotten the last time.{ " " }
       *[none] { "" }
    }{ $asker } stops you: { $fault_what } It has been like that since { $fault_month }, and nobody has come.
ask-fault-need = Get { $fault } in { $fault_hood } repaired (the services desk can move it up the repair queue)
ask-fault-kept = Repaired: { $fault } in { $fault_hood }. { $asker } noticed, and so did the neighbours.
ask-fault-broken = Still broken: { $fault } in { $fault_hood }. { $asker } has stopped expecting you to help.
ask-void-fixed = It was repaired before you answered.
ask-void-fault-gone = What you were asked about is no longer on those streets.
outcome-casework-queued = At the services desk you had { $fault } in { $fault_hood } moved up the repair queue for { $name }.

## The fault view (Today's ask, the street's panel).
ui-faults-here = On these streets
ui-fault-see = See it
ui-fault-see-label = See it: { $fault } in { $hood }
ui-fault-status-open = Waiting for repair
ui-fault-status-repairing = Being repaired
ui-fault-status-fixed = Repaired
ui-fault-since = since { $date }
ui-fault-moved-up = Moved up the repair queue
ui-fault-ahead = Fixed ahead of { $other } in { $other_hood }
ui-fault-put-back = Put back for { $other } in { $other_hood }
ui-fault-next = Next in the repair queue
