while True:
    looked = look('forward')
    if (l := looked[-1][1]) and l.startswith('player'):
        shoot()
        turn_cw()
        move_forward()
        continue

    need_to_turn = len(looked) == 1  # we facing sight blocked

    looked_l = look('left')
    looked_r = look('right')
    if (l := looked_l[-1][1]) and l.startswith('player'):
        turn_ccw()
        shoot()
        need_to_turn = False
        continue
    if (l := looked_r[-1][1]) and l.startswith('player'):
        turn_cw()
        shoot()
        need_to_turn = False
        continue
    if need_to_turn and (len(looked_l) > 1 or len(looked_r) > 1):
        if len(looked_l) > len(looked_r):
            turn_ccw()
        else:
            turn_cw()
        continue
    move_forward()

