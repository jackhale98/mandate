# Text samples (after text audit 1)

About fifty pieces of player-visible text as a player meets them, after the fixes in `TEXT_AUDIT_1.md`, so the owner can judge tone without playing. Each has one line of situation. They come from the rendered corpus (`mandate-cli content doctor --corpus <dir> --seeds 12 --years 4`), rendered by the game's own Fluent code with real arguments.

A conversation reply is composed the way the engine composes one: an opener, sometimes a memory quote, the substance and sometimes a closer. Every part shown is a line that held for that speaker, in their voice, on that day. Bot careers had few conversations, so the parts are drawn from the lines each speaker could say rather than from a played exchange. Voices are in `content/base/conversations/voices.jsonc`; the warmth band is the speaker's opinion of the player (cool below −15, warm from 15, friendly from 50 with trust).

## Conversations

1. **folksy, neutral**: Agnes Roweby, councillor, small talk. Mireenwick; home Westend worries environment, jobs, services; town environment, jobs, housing; you: councillor.
   > Agnes Roweby checks the time, then decides you can have a minute.  
   > "You talked me round on the Parks and green space bill. Don't make a habit of it."  
   > "Did you see the match on Saturday? No, nor did I. I heard it, though. The whole street did."  
   > "And eat something. You look half-starved."  

2. **blunt, warm**: Wim Janssen, plant manager, a pitch for a policy. Taliby; home Hilllands worries jobs, environment, housing; town jobs, housing, environment; you: councillor.
   > "You promised me a favour over the consultancy fee. I'm not chasing. I'm just saying."  
   > "You want my ear on top of what you already owe me? Bold."  

3. **guarded, cool**: Wilma Hale, business association chair, asking for support. Mireenwick; home Harbourgate worries environment, jobs, housing; town environment, jobs, housing; you: councillor.
   > Wilma Hale sees you coming and doesn't pretend otherwise. "Oh. It's you."  
   > "Over the pollution row, you chose the other lot. I'm still a bit sore, if I'm honest."  
   > "Support you? After everything? You've got some neck."  

4. **wry, friendly**: Ulla Brook, resident, about someone or something, about Toma Okaforley. Vellstead; home Vellstead worries housing, taxes, environment; town housing, taxes, services; you: out of office.
   > "Toma Okaforley? Everyone round here knows them. Knowing what they'll do next is another matter."  

5. **plain, warm**: Imre Yateswell, journalist, who runs things. Corriton; home Harbour Green worries housing, taxes, services; town housing, taxes, environment; you: councillor.
   > Imre Yateswell flips a notebook to a clean page, then thinks better of it and closes it.  
   > "If I knew for certain, I'd be printing it, not telling you for nothing."  

6. **smooth, neutral**: Liv Thorneer, retiree, asking for support. Vellenbridge; home Castlefield worries jobs, taxes, services; town jobs, environment, taxes; you: councillor.
   > "You talked me round once. Don't make a habit of it."  
   > "Ask me again when you've paid back what you owe."  

7. **anxious, cool**: Mateo Carrson, teacher, what people worry about. Pellholm; home Kingsheath worries environment, jobs, housing; town jobs, environment, housing; you: councillor.
   > "I hear plenty. It isn't mine to pass on."  

8. **earnest, cool**: Wim Lang, councillor, trading a favour. Mireenwick; home Mireenwick worries environment, jobs, housing; town environment, jobs, housing; you: councillor.
   > Wim Lang meets your eye. "I'll be honest with you, because somebody ought to be."  
   > "A favour? From me? I'd want it in writing first."  
   > "I'm not saying it to be difficult. I'm saying it because it's true."  

9. **pompous, friendly**: Delphine Harrow, community organiser, about someone or something, about Vic Gold. Halobridge; home Churchfield worries housing, taxes, jobs; town housing, environment, taxes; you: councillor.
   > "Vic Gold wants to change everything. Some of it even needs changing."  

10. **wry, neutral**: Osric Shaw, vicar, what people worry about. Larkgate; home Southtown worries services, jobs; town services, jobs, environment; you: councillor.
   > Osric Shaw hears the question all the way through, which almost nobody does.  
   > "Loneliness, if you want the honest answer. It never makes the papers, but it fills my week."  

11. **earnest, warm**: Mara Lang, editor, a pitch for a policy. Mireenwick; home Castlelands worries jobs, environment, housing; town environment, jobs, housing; you: councillor.
   > Mara Lang checks the time, then decides you can have a minute.  
   > "Go on, then. I'm listening."  
   > "Write that down, would you? I'd like somebody to remember."  

12. **blunt, neutral**: Xan Sallow, mayor, who runs things. Mireenwick; home Westend worries environment, jobs, services; town environment, jobs, housing; you: councillor.
   > Xan Sallow looks up from what they were doing. "Yes? What can I do for you?"  
   > "People say nothing much happens without Wilma Hale hearing about it first. That's all I know."  
   > "Don't make me say it twice."  

13. **folksy, warm**: Ulla Kellson, union steward, who runs things. Marford; home Marford worries environment, housing, recovery; town environment, housing, recovery; you: out of office.
   > "Aurel Thatchley. Every time we sit down with management, they've been in the room first. You can tell by how little's left to negotiate."  

14. **plain, cool**: Dita Hartby, shopkeeper, what people worry about. Belenwick; home Belenwick worries jobs, taxes, housing; town jobs, taxes, services; you: councillor.
   > "People tell me all sorts across the counter. It doesn't go any further."  

15. **pompous, hostile**: Arlo Lang, councillor, what people worry about. Corriton; home Corriton worries housing, taxes, environment; town housing, taxes, environment; you: councillor.
   > Arlo Lang offers neither a hand nor a seat.  
   > "If I knew what worried people, I'd have a bigger majority. Ask them yourself."  

16. **smooth, cool**: Greta Shaw, resident, small talk. Mireenwick; home Quarryrow worries environment, jobs, services; town environment, jobs, housing; you: councillor.
   > Greta Shaw smiles as though nothing had ever passed between you. "What a lovely surprise."  
   > "Weather's fine. Anything else?"  
   > "Lovely to see you. Do come again when you've something to offer."  

17. **anxious, warm**: Celia Tallis, business association chair, small talk. Kesenhaven; home Westside worries services, jobs, environment; town jobs, services, environment; you: councillor.
   > "Golf on Sunday, if the rain holds off. It never does."  

18. **guarded, neutral**: Una Esker, party branch chair, a pitch for a policy. Larkgate; home Larkgate worries services, jobs, environment; town services, jobs, environment; you: councillor.
   > "Has the group seen this? Don't bring me things the group hasn't seen."  

19. **pompous, neutral**: Hal Fairson, diner owner, asking for support. Vellenbridge; home Castlefield worries jobs, taxes, services; town jobs, environment, taxes; you: councillor.
   > "I've room in my window for a poster. Whoever fixes the parking gets it."  

20. **smooth, warm**: Tobias Ardenfield, retiree, about someone or something, about the rival. Strathford; home Orchardgate worries jobs, services, housing; town jobs, services, housing; you: out of office.
   > "Rafe Garrowby? Wants to get on, that one. I'd keep an eye over your shoulder."  

21. **blunt, friendly**: Nadia Gallowaywood, councillor, trading a favour. Halobridge; home Halobridge worries environment, housing, taxes; town housing, environment, taxes; you: councillor.
   > "I was going to ring you, actually," says Nadia Gallowaywood, and means it.  
   > "That's what friends are for. I'll still write it down."  

22. **guarded, warm**: Kit Ibarra, nurse, small talk. Vellenbridge; home Vellenbridge worries jobs, environment, taxes; town jobs, environment, taxes; you: councillor.
   > "Don't get me started on jobs. No, really, don't; I'll be here all day."  

23. **earnest, neutral**: Gideon Rasku, vicar, about someone or something, about Taliby. Taliby; home Lowerend worries jobs, housing, taxes; town jobs, housing, taxes; you: councillor.
   > Gideon Rasku hears the question all the way through, which almost nobody does.  
   > "That's not really mine to talk about."  

24. **folksy, neutral**: Agnes Roweby, councillor, what people worry about. Mireenwick; home Westend worries environment, jobs, services; town environment, jobs, housing; you: councillor.
   > Agnes Roweby gives you a polite nod and waits for the question.  
   > "You talked me round on the Parks and green space bill. Don't make a habit of it."  
   > "From what I hear, it's the pollution. People don't talk about much else."  
   > "Mind how you go, love."  

25. **wry, warm**: Rhea Stone, union steward, who runs things. Kesenhaven; home Kingsfield worries jobs, services, environment; town services, jobs, environment; you: councillor.
   > "Me? I just get asked a lot of questions. Like this one."  

26. **anxious, neutral**: Rune Blackwell, mayor, who runs things. Pellholm; home Hillheath worries jobs, environment, housing; town jobs, environment, housing; you: councillor.
   > "There's a name you hear: Saul Goldwell. Whether it's true, I couldn't say."  

27. **plain, friendly**: Aurel Wrenwell, councillor, asking for support. Halobridge; home Abbey Green worries environment, housing, taxes; town housing, environment, taxes; you: councillor.
   > Aurel Wrenwell draws you a little way off, out of earshot.  
   > "You've earned a fair hearing from me. More than fair."  

28. **folksy, cool**: Bodil Alderwell, community organiser, a pitch for a policy. Mireenwick; home Harbourgate worries environment, jobs, housing; town environment, jobs, housing; you: councillor.
   > Bodil Alderwell folds both arms and waits.  
   > "You've a nerve, coming to me with this."  

29. **pompous, warm**: Maeve Carrwood, editor, who runs things. Vellstead; home Queenswood worries housing, taxes, services; town housing, taxes, services; you: councillor.
   > "Gil Sedgewell. Nothing gets decided in this town without someone checking with them first."  

30. **smooth, cool**: Greta Shaw, resident, what people worry about. Mireenwick; home Quarryrow worries environment, jobs, services; town environment, jobs, housing; you: councillor.
   > Greta Shaw smiles as though nothing had ever passed between you. "What a lovely surprise."  
   > "Me? I keep myself to myself."  
   > "Lovely to see you. Do come again when you've something to offer."  

## Dilemmas

31. **plant layoffs** (Halobridge, town worries environment, housing, taxes, crisis: plant_closure; you: councillor).
   **The layoffs at the plant.** Jobs are going in Halobridge, and the town is choosing sides. Anders Nashwell (Plant manager) wants the council to hold off on anything that raises an employer's costs. Anouk Bakr (Union steward) wants you on the picket line. In Wharfvale, people are asking where you stand.  
   - Back Anders Nashwell: no new costs on employers while jobs are at risk  
   - Stand with the workers and draft a hardship fund for laid-off families  
   - Spend a day getting Anders Nashwell and Anouk Bakr into one room  
   - Stay out of it  
   *Afterwards:* You drafted a hardship fund and stood with Anouk Bakr. Laid-off families took heart; Anders Nashwell and the party's donors did not.  

32. **housing squeeze** (Corriton, town worries housing, taxes, environment, crisis: housing_squeeze; you: councillor).
   **The housing squeeze.** Rents in Corriton have outrun wages. Amara Petrovwood (Community organiser) brings families from West Cross who are one rise away from leaving.  
   - Draft an upzoning bill so builders can add homes  
   - Draft an affordable housing programme  
   - Wait for the market to settle  
   *Afterwards:* You drafted upzoning. Renters liked the idea; homeowners worried about towers next door.  

33. **fiscal hole** (Larkgate, town worries services, jobs, environment, crisis: fiscal_hole; you: councillor).
   **The town's budget hole.** Larkgate is borrowing to pay its bills. Mayor Ilka Kaderwell wants cuts and wants your vote for them. Dita Deller (Union steward) says cuts will close services people rely on.  
   - Back the mayor's cuts  
   - Draft a property tax rise instead  
   - Refuse both  
   *Afterwards:* You drafted a tax rise to keep services open. Dita Deller is pleased; better-off homeowners are not.  

34. **amendment for vote** (Mireenwick, town worries environment, jobs, housing; you: councillor).
   **A vote for a project.** Your bill, Local wage floor, is one vote short, and Pia Reed could be that vote. The price: a project in their ward, written into the bill.  
   - Write in their ward project  
   - Offer a four-year sunset clause instead  
   - Let it pass on its merits  
   *Afterwards:* You put a four-year sunset on the Local wage floor bill. Pia Reed is warmer, and the law, if it passes, will need renewing.  

35. **whip deal** (Halobridge, town worries environment, housing, taxes; you: councillor).
   **The party's deal.** Noor Fennwood has a deal: vote with the party on the Transit frequency bill, against your ward's mood, and the party pays toward your campaign.  
   - Take the deal  
   - Haggle: less money, but the whip owes you one  
   - Refuse and vote with your ward  
   - Say nothing; the whip counts you in  
   *Afterwards:* You voted with the party on the Transit frequency bill. The money arrived; your ward noticed which way you went.  

36. **consultancy fee** (Mireenwick, town worries environment, jobs, housing; you: councillor).
   **The consultancy fee.** Sabine Thorne (Plant manager) wants your support at the council and offers a "consultancy fee" for your advice. Cash, no paperwork.  
   - Take the fee  
   - Decline it  
   - Report the offer to the town's auditors  
   - Say nothing and let it pass  
   *Afterwards:* You never answered. Sabine Thorne took the silence as a no.  

37. **rival offer** (Mireenwick, town worries environment, jobs, housing; you: councillor).
   **The truce offer.** Petra Bakr (Candidate) suggests a quiet pact: no attacks from either side until the votes are counted.  
   - Accept the pact  
   - Refuse, in public  
   - Ignore it  
   *Afterwards:* You refused, loudly. The party liked the fight; Petra Bakr will not forget it.  

38. **scandal response** (Corriton, town worries housing, taxes, environment, crisis: housing_squeeze; you: councillor).
   **The newspaper story.** The Corriton Ledger has the story: a cash bribe. How you answer decides how long it runs.  
   - Deny it flatly  
   - Give a full explanation  
   - Apologise in public  
   - Hit back at the paper  
   - Say nothing  
   *Afterwards:* You denied it. If they had little, the story died; if they had a lot, the denial became the story.  

39. **traders meeting** (Vellenbridge, town worries jobs, environment, taxes; you: councillor).
   **The traders' meeting.** Ida Bakrwell (Shopkeeper) is rallying Vellenbridge's traders over the lost jobs and asks you to come and speak.  
   - Go and speak  
   - Send your regrets  
   *Afterwards:* You spoke to the traders. It cost you a day; it won you a friend.  

40. **disaster relief** (Marford, town worries recovery, environment, housing, crisis: disaster_recovery; you: out of office).
   **The rebuilding.** After the disaster, New Green is still clearing up. Kerstin Lowe (Vicar) is running a relief effort out of a borrowed hall and asks what you will do.  
   - Put your own campaign money and a day's work into the relief drive  
   - Draft flood defences, for the long term  
   - Leave it to the town's emergency services  
   *Afterwards:* You spent campaign money and a long day hauling boxes. New Green will remember who turned up.  

## Scenes (at the place, with a quoted memory)

41. **rival offer**, Mireenwick: the speaker remembers something you did.  
   > "You beat me in 2016. Let's keep this civil until the count."

42. **debate**, Mireenwick: the speaker remembers something you did.  
   > "You took the other side over the truce offer. Let the voters see us side by side."

43. **building protest**, Mireenwick: the speaker remembers something you did.  
   > "You stood with me over the building work. Have you seen what they've done to our street?"

44. **pollution**, Mireenwick: the speaker remembers something you did.  
   > "You stood with me over the pollution row. The children here can taste the air."

45. **amendment for vote**, Mireenwick: a first meeting.  
   > "I could vote for your bill. My ward would need a reason to let me."

46. **steer contract**, Mireenwick: a first meeting.  
   > Jasper Garrow: "I support people who support this town."

## Overheard

47. **news.headline**, union hall in Quarryrow, Mireenwick (town worries environment, jobs, housing).  
   > "Did you see The Mireenwick Chronicle? 'Local wage floor bill passes in Mireenwick.' It's all anyone's on about."

48. **jobs.industry**, Fen's (diner) in Mireenwick, Mireenwick (town worries environment, jobs, housing).  
   > "Everything round here comes back to the factories. One bad year there and the whole of Mireenwick feels it."

49. **housing.viewing**, market in Corriton, Corriton (town worries housing, taxes, environment, crisis: housing_squeeze).  
   > "Forty people turned up to view one flat in Corriton. Forty."

50. **recovery.hall**, union hall in North Quarter, Marford (town worries recovery, environment, housing, crisis: disaster_recovery).  
   > "The church hall's still feeding people. The council? Nowhere to be seen."

51. **scandal.player_defended**, Odile's (diner) in Corriton, Corriton (town worries housing, taxes, environment, crisis: housing_squeeze).  
   > "All this about Wim Hart and a cash bribe. The papers will print anything."

52. **taxes.vote**, Linus's (diner) in Vellenbridge, Vellenbridge (town worries jobs, environment, taxes).  
   > "Whoever promises to cut the rates gets my vote. Simple as that."

## Press

53. **headline-policy-defeated** (Mireenwick).  
   > Small business grants bill voted down in Mireenwick, 2–5

54. **headline-player-elected** (Vellenbridge).  
   > Celia Janssenfield wins in Vellenbridge

55. **headline-response-deny** (Corriton).  
   > Wim Hart denies a cash bribe

## Asked about a place or a neighbourhood (the relevance pass)

56. **journalist, neutral**: "Ask about…" the paper they work for, The Corriton Ledger. (Before this pass the reply was about the rent.)  
   > "The editor at The Corriton Ledger reads every line twice and cuts half. You'd be amazed who rings up asking us not to run something. Some of them sit on the council."

57. **union steward, warm**: "Ask about…" the union hall, where they work.  
   > "You mean the union hall? I've the keys, the kettle and the grievances. Thursday nights it's standing room only, and everyone wants to know what management's planning."

58. **resident, neutral**: "Ask about…" the town hall, which is not theirs.  
   > "You mean the town hall? Forms in, money out, and a queue either way. Everyone ends up there sooner or later."

59. **councillor, neutral**: "What are people worried about?", asked of Castlelands, which is not their end of town.  
   > "Castlelands? Not my end of town, so I'd only be guessing. Ask someone who lives there."

60. **resident, warm**: the same question of Castlelands, their home (top worry: jobs; the notebook then says the same).  
   > "Castlelands? I'd know; it's home. It's jobs, and you could set your watch by the complaints."

