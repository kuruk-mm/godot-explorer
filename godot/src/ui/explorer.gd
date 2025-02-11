extends Node

var scene_runner: SceneManager = null
var realm: Realm = null
var parcel_manager: ParcelManager = null

@onready var label_crosshair = $UI/Label_Crosshair
@onready var control_pointer_tooltip = $UI/Control_PointerTooltip

@onready var panel_chat = $UI/VBoxContainer_Chat/MarginContainer/Panel_Chat

@onready var label_fps = %Label_FPS
@onready var label_ram = %Label_RAM
@onready var control_menu = $UI/Control_Menu
@onready var control_minimap = $UI/Control_Minimap
@onready var player := $Player
@onready var mobile_ui = $UI/MobileUI
@onready var v_box_container_chat = $UI/VBoxContainer_Chat
@onready var button_jump = $UI/Button_Jump

var parcel_position: Vector2i
var _last_parcel_position: Vector2i
var parcel_position_real: Vector2
var panel_bottom_left_height: int = 0
var dirty_save_position: bool = false


func _process(_dt):
	parcel_position_real = Vector2(player.position.x * 0.0625, -player.position.z * 0.0625)
	control_minimap.set_center_position(parcel_position_real)

	parcel_position = Vector2i(floori(parcel_position_real.x), floori(parcel_position_real.y))
	if _last_parcel_position != parcel_position:
		parcel_manager.update_position(parcel_position)
		_last_parcel_position = parcel_position
		Global.config.last_parcel_position = parcel_position
		dirty_save_position = true


func _on_parcels_procesed(parcels, empty):
	control_minimap.control_map_shader.set_used_parcels(parcels, empty)
	control_menu.control_map.control_map_shader.set_used_parcels(parcels, empty)


func _ready():
	if Global.is_mobile:
		v_box_container_chat.alignment = VBoxContainer.ALIGNMENT_BEGIN
		mobile_ui.show()
		label_crosshair.show()
	else:
		v_box_container_chat.alignment = VBoxContainer.ALIGNMENT_END
		mobile_ui.hide()
		button_jump.hide()

	var sky = null
	match Global.config.skybox:
		0:
			sky = load("res://assets/sky/sky_basic.tscn").instantiate()
		1:
			sky = load("res://assets/sky/krzmig/world_environment.tscn").instantiate()
			sky.day_time = 14.9859

	add_child(sky)
	if Global.config.skybox == 1:
		sky.day_time = 10

	control_pointer_tooltip.hide()
	var start_parcel_position: Vector2i = Vector2i(Global.config.last_parcel_position)
	player.position = 16 * Vector3(start_parcel_position.x, 0.1, -start_parcel_position.y)
	player.look_at(16 * Vector3(start_parcel_position.x + 1, 0, -(start_parcel_position.y + 1)))

	scene_runner = get_tree().root.get_node("scene_runner")
	scene_runner.set_camera_and_player_node(player.camera, player, self._on_scene_console_message)
	scene_runner.pointer_tooltip_changed.connect(self._on_pointer_tooltip_changed)

	realm = get_tree().root.get_node("realm")

	parcel_manager = ParcelManager.new()
	add_child(parcel_manager)
	parcel_manager.connect("parcels_processed", self._on_parcels_procesed)

	if Global.config.last_realm_joined.is_empty():
		realm.set_realm("https://sdk-team-cdn.decentraland.org/ipfs/goerli-plaza-main")
	else:
		realm.set_realm(Global.config.last_realm_joined)

	self.scene_runner.process_mode = Node.PROCESS_MODE_INHERIT

	control_menu.control_advance_settings.preview_hot_reload.connect(
		self._on_panel_bottom_left_preview_hot_reload
	)


func _on_scene_console_message(scene_id: int, level: int, timestamp: float, text: String) -> void:
	_scene_console_message.call_deferred(scene_id, level, timestamp, text)


func _scene_console_message(scene_id: int, level: int, timestamp: float, text: String) -> void:
	var title: String = scene_runner.get_scene_title(scene_id)
	title += str(scene_runner.get_scene_base_parcel(scene_id))
	control_menu.control_advance_settings._on_console_add(title, level, timestamp, text)


func _on_pointer_tooltip_changed():
	change_tooltips.call_deferred()


func change_tooltips():
	var tooltips = scene_runner.get_tooltips()

	if not tooltips.is_empty():
		control_pointer_tooltip.set_pointer_data(tooltips)
		control_pointer_tooltip.show()
	else:
		control_pointer_tooltip.hide()


func _on_check_button_toggled(button_pressed):
	scene_runner.set_pause(button_pressed)


func _on_ui_gui_input(event):
	if not Global.is_mobile:
		if event is InputEventMouseButton and event.pressed:
			if Input.mouse_mode != Input.MOUSE_MODE_CAPTURED:
				Input.set_mouse_mode(Input.MOUSE_MODE_CAPTURED)
				label_crosshair.show()


@onready var line_edit_command = $UI/LineEdit_Command


func _unhandled_input(event):
	if not Global.is_mobile:
		if event is InputEventKey:
			if event.pressed and event.keycode == KEY_TAB:
				if not control_menu.visible:
					control_menu.show_last()
					Input.set_mouse_mode(Input.MOUSE_MODE_VISIBLE)
					label_crosshair.hide()

			if event.pressed and event.keycode == KEY_M:
				if control_menu.visible:
					pass
				else:
					control_menu.show_map()
					Input.set_mouse_mode(Input.MOUSE_MODE_VISIBLE)
					label_crosshair.hide()

			if event.pressed and event.keycode == KEY_ESCAPE:
				if Input.mouse_mode == Input.MOUSE_MODE_CAPTURED:
					Input.set_mouse_mode(Input.MOUSE_MODE_VISIBLE)
					label_crosshair.hide()

				line_edit_command.finish()

			if event.pressed and event.keycode == KEY_ENTER:
				line_edit_command.start()


func _toggle_ram_usage(visibility: bool):
	if visibility:
		label_ram.show()
	else:
		label_ram.hide()


func _on_control_minimap_request_open_map():
	if !control_menu.visible:
		control_menu.show_map()
		Input.set_mouse_mode(Input.MOUSE_MODE_VISIBLE)
		label_crosshair.hide()


func _on_control_menu_jump_to(parcel: Vector2i):
	player.set_position(Vector3i(parcel.x * 16, 3, -parcel.y * 16))
	control_menu.close()


func _on_control_menu_hide_menu():
	control_menu.close()
	control_menu.control_map.clear()


func _on_timer_timeout():
	label_ram.set_text("RAM Usage: " + str(OS.get_static_memory_usage() / 1024.0 / 1024.0) + " MB")
	label_fps.set_text(str(Engine.get_frames_per_second()) + " FPS")
	if dirty_save_position:
		dirty_save_position = false
		Global.config.save_to_settings_file()


func _on_control_menu_toggle_ram(visibility):
	label_ram.visible = visibility


func _on_control_menu_toggle_fps(visibility):
	label_fps.visible = visibility


func _on_control_menu_toggle_minimap(visibility):
	control_minimap.visible = visibility


func _on_panel_bottom_left_preview_hot_reload(_scene_type, scene_id):
	parcel_manager.reload_scene(scene_id)


var last_position_sent: Vector3 = Vector3.ZERO
var counter: int = 0


func _on_timer_broadcast_position_timeout():
	var transform: Transform3D = player.avatar.global_transform
	var position = transform.origin
	var rotation = transform.basis.get_rotation_quaternion()

	if last_position_sent.is_equal_approx(position):
		counter += 1
		if counter < 10:
			return

	Global.comms.broadcast_position_and_rotation(position, rotation)
	last_position_sent = position
	counter = 0


func _on_virtual_joystick_right_stick_position(stick_position: Vector2):
	player.stick_position = stick_position


func _on_virtual_joystick_right_is_hold(hold: bool):
	player.stick_holded = hold


func _on_touch_screen_button_pressed():
	Input.action_press("ia_jump")


func _on_touch_screen_button_released():
	Input.action_release("ia_jump")


func _on_line_edit_command_submit_message(message: String):
	line_edit_command.finish()

	if message.length() == 0:
		return

	var params := message.split(" ")
	var command_str := params[0].to_lower()
	if command_str.begins_with("/"):
		if command_str == "/go" or command_str == "/goto" and params.size() > 1:
			var comma_params = params[1].split(",")
			var dest_vector = Vector2i(0, 0)
			if comma_params.size() > 1:
				dest_vector = Vector2i(int(comma_params[0]), int(comma_params[1]))
			elif params.size() > 2:
				dest_vector = Vector2i(int(params[1]), int(params[2]))

			panel_chat.add_chat_message(
				"[color=#ccc]> Teleport to " + str(dest_vector) + "[/color]"
			)
			_on_control_menu_jump_to(dest_vector)
		elif command_str == "/changerealm" and params.size() > 1:
			panel_chat.add_chat_message(
				"[color=#ccc]> Trying to change to realm " + params[1] + "[/color]"
			)
			realm.set_realm(params[1])
		else:
			pass
			# TODO: unknown command
	else:
		Global.comms.send_chat(message)
		panel_chat._on_chats_arrived([["Godot User", 0, message]])


func _on_control_menu_request_pause_scenes(enabled):
	scene_runner.set_pause(enabled)


func _on_button_jump_gui_input(event):
	if event is InputEventScreenTouch:
		Input.action_press("ia_jump")
